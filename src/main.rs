use serde::{Serialize, Deserialize};
use stateright::actor::{*, register::*};
use std::borrow::Cow;
use std::net::{SocketAddrV4, Ipv4Addr};


type Value = Vec<u8>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct PaxosState {
    last_seen: Option<PaxosMsg>
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
    id: Id 
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PaxosMsg {
    Prepare(RoundIdentifier),
    Promise(RoundIdentifier),
    Accept(RoundIdentifier, Vec<u8>),
    Accepted(RoundIdentifier, Vec<u8>),
    Nack(RoundIdentifier)
}

pub struct PaxosActor {
    peers: Vec<Id>,
}

impl Actor for PaxosActor {
    type Msg = PaxosMsg;
    type State = PaxosState;

    fn on_start(&self, id: Id, o: &mut Out<Self>) -> Self::State {
        unimplemented!()
    }
    fn on_msg(&self, id: Id, state: &mut Cow<Self::State>, src: Id, msg: Self::Msg, o: &mut Out<Self>) {
        unimplemented!()
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use stateright::{*, semantics::*, semantics::register::*};
    use ActorModelAction::Deliver;
    use RegisterMsg::{Get, GetOk, Put, PutOk};

}

fn main() {
    println!("Hello, world!");
}
