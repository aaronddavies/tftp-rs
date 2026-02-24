use crate::constants::MAX_PACKET_SIZE;
use crate::errors::TftprsError;

trait State {
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

struct StateMachine {
    state: Box<dyn State>,
    terminated: bool,
}
