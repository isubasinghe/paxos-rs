use serde::{Deserialize, Serialize};
use stateright::actor::{register::*, *};
use stateright::semantics::LinearizabilityTester;
use stateright::semantics::register::Register;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::net::{Ipv4Addr, SocketAddrV4};

type RegisterValue = (i32, i32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct PaxosState {
    id: Id,
    round: u32,
    prepare_data: BTreeMap<RoundIdentifier, RegisterValue>,
    promises: BTreeMap<RoundIdentifier, BTreeSet<Id>>,
    accepts: BTreeMap<RoundIdentifier, BTreeSet<Id>>,
    last_seen: Option<RoundIdentifier>,
}

impl PaxosState {
    fn next_round(&self) -> RoundIdentifier {
        RoundIdentifier {
            id: self.id.clone(),
            round_num: self.round + 1,
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

impl PartialEq<Option<RoundIdentifier>> for RoundIdentifier {
    fn eq(&self, other: &Option<RoundIdentifier>) -> bool {
        let rid = match other {
            Some(rid) => rid,
            None => return false,
        };
        self.eq(rid)
    }
}

impl PartialOrd<Option<RoundIdentifier>> for RoundIdentifier {
    fn partial_cmp(&self, other: &Option<RoundIdentifier>) -> Option<std::cmp::Ordering> {
        other.map(|val| self.cmp(&val))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PaxosMsg {
    Prepare(RoundIdentifier),
    Promise(RoundIdentifier),
    Accept(RoundIdentifier, RegisterValue),
    Accepted(RoundIdentifier, RegisterValue),
}

pub struct PaxosActor {
    peers: Vec<Id>,
}

impl Actor for PaxosActor {
    type Msg = RegisterMsg<RoundIdentifier, RegisterValue, PaxosMsg>;
    type State = PaxosState;

    fn on_start(&self, id: Id, _o: &mut Out<Self>) -> Self::State {
        PaxosState {
            id,
            round: 0,
            prepare_data: BTreeMap::new(),
            last_seen: None,
            promises: BTreeMap::new(),
            accepts: BTreeMap::new(),
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
                    PaxosMsg::Prepare(rid) => {
                        if rid > state.last_seen {
                            let state = state.to_mut();
                            state.last_seen = Some(rid);
                            let msg = RegisterMsg::Internal(PaxosMsg::Promise(rid));
                            o.send(src, msg);
                        } else {
                            // nack
                        }
                    }
                    PaxosMsg::Promise(rid) => {
                        let state = state.to_mut();
                        match state.promises.get_mut(&rid) {
                            Some(set) => {
                                set.insert(rid.id);
                            }
                            None => {
                                let mut set = BTreeSet::new();
                                set.insert(rid.id);
                                state.promises.insert(rid, set);
                            }
                        };

                        let (key, value) = match state.prepare_data.get(&rid) {
                            Some(data) => *data,
                            None => return,
                        };

                        let count = match state.promises.get(&rid) {
                            Some(s) => s.len(),
                            None => 0,
                        };
                        let num_peers = self.peers.len();
                        // we have a majority
                        if count / 2 > num_peers {
                            let msg = RegisterMsg::Internal(PaxosMsg::Accept(rid, (key, value)));
                            o.send(src, msg);
                        }
                    }
                    PaxosMsg::Accept(rid, (key, value)) => {
                        if Some(rid) == state.last_seen {
                            let msg = RegisterMsg::Internal(PaxosMsg::Accepted(rid, (key, value)));
                            o.broadcast(&self.peers, &msg);
                        }
                    }
                    PaxosMsg::Accepted(rid, (key, value)) => {
                        let state = state.to_mut();

                        match state.accepts.get_mut(&rid) {
                            Some(set) => {
                                set.insert(rid.id);
                            }
                            None => {
                                let mut set = BTreeSet::new();
                                set.insert(rid.id);
                                state.accepts.insert(rid, set);
                            }
                        };

                        let count = match state.accepts.get(&rid) {
                            Some(s) => s.len(),
                            None => 0,
                        };

                        let num_peers = self.peers.len();
                        if count / 2 > num_peers {
                            // let msg = RegisterMsg::PutOk(rid);   
                            
                        }
                    }
                }
            }
            RegisterMsg::Put(rid, (key, value)) => {
                let state = state.to_mut();
                state.prepare_data.insert(rid, (key, value));
                let msg = RegisterMsg::Internal(PaxosMsg::Prepare(state.next_round()));
                o.broadcast(&self.peers, &msg);
            }, 
            _ => {}
        }
    }
}

#[derive(Clone)]
struct PaxosModelConfig {
    client_count: usize,
    server_count: usize
}

impl PaxosModelConfig {

    fn into_model(self) {
        let _ = RegisterActor::from
        let _ =  ActorModel::new(self.clone(), ());
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use stateright::{semantics::register::*, semantics::*, *};
    use ActorModelAction::Deliver;
    use RegisterMsg::{Get, GetOk, Put, PutOk};

}

fn main() {
    println!("Hello, world!");
}
