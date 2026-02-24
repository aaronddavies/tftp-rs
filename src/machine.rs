use crate::constants::MAX_PACKET_SIZE;
use crate::constants::DEFAULT_DESTINATION_TID;
use crate::errors::TftprsError;

use rand::rngs::SmallRng;
use rand::Rng;

trait State {
    fn send(&mut self, data: Vec<u8>);
    fn receive(&mut self, message: &[u8; MAX_PACKET_SIZE]) -> Result<Box<dyn State>, TftprsError>;
}

struct ListenState {

}

impl ListenState {
    fn new() -> ListenState {
        ListenState{}
    }
}

impl State for ListenState {
    fn receive(&mut self, message: &[u8; MAX_PACKET_SIZE]) -> Result<Box<dyn State>, TftprsError> {
        Ok(Box::new(ListenState::new()))
    }
}

struct Machine {
    state: Box<dyn State>,
    terminated: bool,
    source: u16,
    destination: u16,
}

impl Machine {
    fn new() -> Machine {
        let mut rng :SmallRng = rand::make_rng();
        let source: u16 = rng.next_u32() as u16;
        Self {
            state: Box::new(ListenState::new()),
            terminated: false,
            source,
            destination: DEFAULT_DESTINATION_TID
        }
    }
}
