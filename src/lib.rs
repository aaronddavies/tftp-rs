pub(crate) mod constants;
pub mod errors;
pub mod machine;
pub(crate) mod serial;

mod tests {
    #[cfg(test)]
    use crate::machine::*;
    #[cfg(test)]
    use crate::constants::*;
    #[cfg(test)]
    use crate::serial::*;

    #[test]
    fn test_write_request() {
        let mut machine = Machine::new();
        assert!(!machine.is_busy());
        let mut my_file: Vec<u8> = [0x5A; 1024].to_vec();
        let mut tx = [0u8; MAX_PACKET_SIZE];
        let mut rx = [0u8; MAX_PACKET_SIZE];
        // Send request
        let count =machine.request_send_file(String::from("ABCDE"), &mut my_file, &mut tx).expect("send file");
        assert_eq!(count, 14);
        assert_eq!(tx[1], OpCode::WriteRequest as u8);
        assert_eq!(machine.request_type().unwrap(), RequestType::Write);
        assert!(machine.is_busy());
        assert_eq!(machine.mode(), Mode::Binary);
        // Process ack
        let ack = Ack::new(0);
        let count = ack.serialize(&mut rx);
        let count = machine.process(&rx, count, &mut tx).unwrap();
        // Send out next packet
        assert_eq!(count, MAX_PACKET_SIZE);
        assert_eq!(tx[1], OpCode::Data as u8);
    }

    #[test]
    fn test_read_request() {
        // For incoming file
        let mut my_file: Vec<u8> = Vec::new();
        // For sendimg outgoing messages to server
        let mut tx = [0u8; MAX_PACKET_SIZE];
        // For capturing incoming messages from server
        let mut rx = [0u8; MAX_PACKET_SIZE];

        // Throw away the machine when the transaction is done so that we can inspect the file
        {
            let mut machine = Machine::new();
            // Send request
            let count = machine.request_receive_file(String::from("ABCDE"), &mut my_file, &mut tx).expect("receive file");
            assert_eq!(count, 14);
            assert_eq!(tx[1], OpCode::ReadRequest as u8);

            // Server's first block message
            let mut incoming_data = [0x5A; 1024].to_vec();
            // Disambiguate the two blocks
            incoming_data[MAX_DATA_SIZE] = 0xA5;

            // Process first block
            let data = Data::new(1, &incoming_data);
            data.unwrap().serialize(&mut rx);
            let count = machine.process(&rx, MAX_PACKET_SIZE, &mut tx).unwrap();

            // Send ack
            assert_eq!(count, 4);
            assert_eq!(tx[1], OpCode::Acknowledgement as u8);
            assert_eq!(tx[3], 1);

            // Process second block
            let data = Data::new(2, &incoming_data);
            data.unwrap().serialize(&mut rx);
            let count = machine.process(&rx, MAX_PACKET_SIZE, &mut tx).unwrap();

            // Send ack
            assert_eq!(count, 4);
            assert_eq!(tx[1], OpCode::Acknowledgement as u8);
            assert_eq!(tx[3], 2);
        }

        // Verify data written
        assert_eq!(my_file.get(0).unwrap(), &0x5A);
        assert_eq!(my_file.get(MAX_DATA_SIZE - 1).unwrap(), &0x5A);
        assert_eq!(my_file.get(MAX_DATA_SIZE).unwrap(), &0xA5);
        assert_eq!(my_file.len(), MAX_DATA_SIZE * 2);
    }

    #[test]
    fn test_listen_for_read_request() {
        // For outgoing file
        let my_file: Vec<u8> = Vec::new();
        // For outgoing messages to server
        let mut tx = [0u8; MAX_PACKET_SIZE];
        // For capturing incoming messages from server
        let mut rx = [0u8; MAX_PACKET_SIZE];

        {
            let mut machine = Machine::new();
            // Send request
            let request = Request::new(RequestType::Read, Mode::Binary, String::from("ABCDE"));
            request.unwrap().serialize(&mut rx);
            let filename = machine.listen_for_request(&rx).unwrap();
            assert_eq!(machine.request_type().unwrap(), RequestType::Write);
            assert!(machine.is_busy());
            assert_eq!(machine.mode(), Mode::Binary);
            assert_eq!(filename, String::from("ABCDE"));
            let count = machine.reply_send_file(&my_file, &mut tx);
        }
    }
}
