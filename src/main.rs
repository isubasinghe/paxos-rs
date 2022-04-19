use serde::{Deserialize, Serialize};
use stateright::actor::{register::*, *};
use stateright::semantics::register::Register;
use stateright::semantics::LinearizabilityTester;
use std::borrow::Cow;
use stateright::Model;
use std::collections::{BTreeMap, BTreeSet};
use std::net::{Ipv4Addr, SocketAddrV4};

type RegisterValue = char;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct PaxosState {
    id: Id,
    round: u32,
    prepare_data: BTreeMap<RoundIdentifier, RegisterValue>,
    promises: BTreeMap<RoundIdentifier, BTreeSet<Id>>,
    accepts: BTreeMap<RoundIdentifier, BTreeSet<Id>>,
    last_seen: Option<RoundIdentifier>,
    value: Option<char>,
    decided: bool
}

impl PaxosState {
    fn next_round(&mut self) -> RoundIdentifier {
        self.round += 1;
        RoundIdentifier {
            id: self.id.clone(),
            round_num: self.round,
        }
    }
}

// strategy to make forward progress on Paxos
// "majority wins" is not needed for linearizability only for the strict (arguably correct) definition of "consensus".
trait ForwardStrategy {
    fn majority_promises() -> bool;
    fn majority_acceptor() -> bool;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct RoundIdentifier {
    round_num: u32,
    id: Id,
}

impl PartialOrd for RoundIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for RoundIdentifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.round_num != other.round_num {
            return self.round_num.cmp(&other.round_num);
        }
        return self.id.cmp(&other.id);
    }
}



#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PaxosMsg {
    Prepare(u64, Id, RoundIdentifier),
    Promise(u64, Id, RoundIdentifier), 
    Accept(u64, Id, RoundIdentifier, RegisterValue),
    Accepted(u64, Id, RoundIdentifier, RegisterValue),
}

pub struct PaxosActor {
    peers: Vec<Id>,
}

impl Actor for PaxosActor {
    type Msg = RegisterMsg<u64, RegisterValue, PaxosMsg>;
    type State = PaxosState;

    fn on_start(&self, id: Id, _o: &mut Out<Self>) -> Self::State {
        PaxosState {
            id,
            round: 0,
            prepare_data: BTreeMap::new(),
            last_seen: None,
            promises: BTreeMap::new(),
            accepts: BTreeMap::new(),
            decided: false, 
            value: None
        }
    }
    fn on_msg(
        &self,
        _: Id,
        state: &mut Cow<Self::State>,
        src: Id,
        msg: Self::Msg,
        o: &mut Out<Self>,
    ) {
        match msg {
            RegisterMsg::Internal(internal_msg) => {
                match internal_msg {
                    // request_id is stateright specific while rid is the round identifier
                    PaxosMsg::Prepare(request_id, org_sender, rid) => {
                        if state.decided {
                            return;
                        }

                        let greater = match state.last_seen {
                            Some(val) => rid > val,
                            None => true
                        };
                        if greater {
                            let state = state.to_mut();
                            state.last_seen = Some(rid);
                            let msg = RegisterMsg::Internal(PaxosMsg::Promise(request_id, org_sender, rid));
                            o.send(src, msg);
                        } else {
                            // nack
                        } 
                    }

                    // request_id is stateright specific while rid is the round identifier
                    PaxosMsg::Promise(request_id, org_sender, rid) => {
                        if state.decided {
                            return 
                        }
                        let state = state.to_mut();
                        match state.promises.get_mut(&rid) {
                            Some(set) => {
                                set.insert(src);
                            }
                            None => {
                                let mut set = BTreeSet::new();
                                set.insert(src);
                                state.promises.insert(rid, set);
                            }
                        };

                        let value = match state.prepare_data.get(&rid) {
                            Some(data) => *data,
                            None => return,
                        };

                        let count = match state.promises.get(&rid) {
                            Some(s) => s.len(),
                            None => 0,
                        };
                        let num_peers = self.peers.len();
                        // we have a majority
                        if count  > num_peers / 2 {
                            let msg =
                                RegisterMsg::Internal(PaxosMsg::Accept(request_id, org_sender, rid, value));
                            o.broadcast(&self.peers, &msg);
                        }
                    }
                    PaxosMsg::Accept(request_id, org_sender, rid, value) => {
                        if state.decided {
                            return;
                        }
                        if Some(rid) == state.last_seen {
                            let msg =
                                RegisterMsg::Internal(PaxosMsg::Accepted(request_id, org_sender, rid, value));
                            o.broadcast(&self.peers, &msg);
                        }
                    }
                    PaxosMsg::Accepted(request_id, org_sender, rid, value) => {
                        if state.decided {
                            return; 
                        }
                        let state = state.to_mut();

                        match state.accepts.get_mut(&rid) {
                            Some(set) => {
                                set.insert(src);
                            }
                            None => {
                                let mut set = BTreeSet::new();
                                set.insert(src);
                                state.accepts.insert(rid, set);
                            }
                        };

                        let count = match state.accepts.get(&rid) {
                            Some(s) => s.len(),
                            None => 0,
                        };

                        let num_peers = self.peers.len();
                        if count > num_peers / 2 {
                            let msg = RegisterMsg::PutOk(request_id);
                            state.value = Some(value);
                            state.decided = true;
                            o.send(org_sender, msg);
                            println!("{} has accepted value {}", state.id, value);
                            
                            
                        }
                    }
                }
            }
            RegisterMsg::Put(request_id, value) => {
                let state = state.to_mut();
                let rid = state.next_round();
                state.prepare_data.insert(rid, value);
                let msg = RegisterMsg::Internal(PaxosMsg::Prepare(request_id, src, rid));
                o.broadcast(&self.peers, &msg);
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
struct PaxosModelConfig {
    client_count: usize,
    server_count: usize,
}

impl PaxosModelConfig {
    fn into_model(
        self,
    ) -> ActorModel<
        RegisterActor<PaxosActor>,
        Self,
        LinearizabilityTester<Id, Register<RegisterValue>>,
    > {
        ActorModel::new(
            self.clone(),
            LinearizabilityTester::new(Register(RegisterValue::default())),
        )
        .actors((0..self.server_count).map(|i| {
            RegisterActor::Server(PaxosActor {
                peers: model_peers(i, self.server_count),
            })
        }))
        .actors((0..self.client_count).map(|i| RegisterActor::Client {
            put_count: 1,
            server_count: self.server_count,
        }))
        .duplicating_network(DuplicatingNetwork::No)
        .property(
            stateright::Expectation::Always,
            "linearizable",
            |_, state| state.history.serialized_history().is_some(),
        )
        .property(
            stateright::Expectation::Sometimes,
            "value chosen",
            |_, state| {
                for env in &state.network {
                    if let RegisterMsg::GetOk(_, value) = env.msg {
                        if value != RegisterValue::default() {
                            return true;
                        }
                    }
                }
                false
            },
        )
        .record_msg_in(RegisterMsg::record_returns)
        .record_msg_out(RegisterMsg::record_invocations)
    }
}
/*
#[cfg(test)]
mod test {
    use super::*;
    use stateright::{semantics::register::*, semantics::*, *};
    use ActorModelAction::Deliver;
    use RegisterMsg::{Get, GetOk, Put, PutOk};
}
 */
fn main() {
    let address = "localhost:3000";  
    let clients = 3;
    println!("Serving from {0} for {1} client(s)", address, clients);
    PaxosModelConfig{
        client_count: clients, 
        server_count: 3, 
    }.into_model().checker().threads(12).serve(address);

}
