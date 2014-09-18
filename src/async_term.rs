use std::io::stdio;
use std::str;
use std::sync::Future;
use zmq;
use zmq::{Context, PollItem, Socket};


static ZMQ_ENDPOINT : &'static str = "inproc://async_terminal";


pub struct Terminal {
    socket : Socket,
    future : Future<()>,
}


impl Terminal {
    pub fn new(zmq_context : &mut Context) -> Terminal {
        let mut public_socket = zmq_context.socket(zmq::PAIR).unwrap();
        public_socket.bind(ZMQ_ENDPOINT).unwrap();

        let mut private = TerminalPrivate::new(zmq_context, ZMQ_ENDPOINT);

        let future = Future::spawn(proc() {
            private.worker()
        });
        public_socket.recv_str(0).unwrap();

        Terminal {
            socket : public_socket,
            future : future
        }
    }

    pub fn poll_line(&mut self) -> Option<String> {
        match zmq::poll([self.socket.as_poll_item(zmq::POLLIN)], 0) {
            Ok(1) => Some(self.get_line()),
            _ => None
        }
    }

    pub fn get_line(&mut self) -> String { self.socket.recv_str(0).unwrap() }

    pub fn shutdown(&mut self) {
        self.socket.send_str("QUIT", 0).unwrap();
        self.future.get();
    }
}


struct TerminalPrivate<'a> {
    socket         : Socket,
    stdin_buffer   : [u8, ..1024],
    poll_items     : [PollItem<'a>, ..2],
}


impl<'a> TerminalPrivate<'a> {
    fn new(zmq_context : &mut Context, endpoint : &str)
            -> TerminalPrivate<'a> {
        let mut socket = zmq_context.socket(zmq::PAIR).unwrap();
        let poll_items = [socket.as_poll_item(zmq::POLLIN),
                          PollItem::fd(0, zmq::POLLIN)];
        socket.connect(endpoint).unwrap();

        TerminalPrivate {
            socket: socket,
            stdin_buffer: [0u8, ..1024],
            poll_items: poll_items,
        }
    }

    fn handle_message(&mut self) -> bool {
        if !is_ready(&self.poll_items[0]) { return false; }
        match self.socket.recv_str(0) {
            Ok(msg) => match msg.as_slice() {
                "QUIT" => {
                    println!("console: shutdown requested.");
                    true
                },
                _ => {
                    println!("console: unknown message '{}'", msg);
                    false
                }
            },
            Err(err) => {
                println!("console: zmq error '{}'", err);
                true
            }
        }
    }

    fn handle_stdin(&mut self) -> bool {
        if !is_ready(&self.poll_items[1]) { return false; }
        let n_bytes = match stdio::stdin_raw().read(&mut self.stdin_buffer) {
            Ok(n) => n,
            Err(err) => {
                println!("console: stdin error: {}", err);
                self.socket.send_str("exit", 0).unwrap();
                self.wait_for_quit();
                return true;
            }
        };
        let new_buf = self.stdin_buffer.slice_to(n_bytes);
        for line in str::from_utf8(new_buf).unwrap().lines_any() {
            self.socket.send_str(line, 0).unwrap();
        }

        false
    }

    fn worker(&mut self) {
        self.socket.send_str("UP", 0).unwrap();

        loop {
            let poll_result = zmq::poll(&mut self.poll_items, -1);
            match poll_result {
                Ok(_) => {
                   if self.handle_message() { break; }
                   if self.handle_stdin() { break; }
                },
                Err(err) => {
                    println!("console: zmq error {}", err);
                    self.wait_for_quit();
                    break;
                }
            }
        }
    }

    fn wait_for_quit(&mut self) {
        let _ = self.socket.recv_str(0);
    }
}

fn is_ready(item : &PollItem) -> bool { item.get_revents() & zmq::POLLIN != 0 }
