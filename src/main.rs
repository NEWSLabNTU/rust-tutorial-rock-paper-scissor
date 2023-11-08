//! A rock-paper-scissor gameplay client to demonstrate Rust's
//! async/.await feature.
//!
//! The code is written in a more readable way along with line-by-line
//! comments. The purpose is to help newcomers to understand the usage
//! of async/.await syntax and learn concurrent programming.
//!
//! The implemented program connects to the opponent via an UDP
//! socket, reads user's and opponent's moves simultanously, and
//! determines the winner.
//!
//! The step to read both players' moves simultanously is the place
//! eleases the power of Rust. It has to wait for stdin input and
//! socket data in the mean time. The challenging part is that if the
//! program solely blocks on the stdin input, it may misses the
//! message from the remote user.
//!
//! In in old days of C/C++, we use select() to poll on the stdin and
//! socket and wait until one of them to become ready. However, the
//! time selects() saying stdin is ready does not mean that the input
//! is complete. The program may read an incomplete message from the
//! user. It has to store the state of the input and select() again to
//! wait for the remaining part. It runs into the hassle to write
//! state machine to keep track of the the local and remote user input
//! states.
//!
//! ```c
//! while (1) {
//!     fd_set rfds;
//!     FD_SET(0, &rfds);     // stdin
//!     FD_SET(sockfd, &rfds);  // socket
//!
//!     int ret = select(2, &rfds, NULL, NULL, NULL);
//!     if (ret < 0) { exit(1); }
//!
//!     if (FD_ISSET(0, &rfds)) {
//!         // Process stdin input
//!     } else if (FD_ISSET(sockfd, &rfds)) {
//!         // Process socket data
//!     }
//! }
//! ```
//!
//! One common alternative is to use threads. We can spawn two
//! threads, one for the local stdin and the other for the
//! socket. This way simplifies the programming of concurrent
//! operations. However, it comes with costs. Spawning threads
//! introduces overheads, and on some embedded systems, threading is
//! not supported.
//!
//! ```c
//! void *stdin_worker(void* args) { /* Process stdin input */ }
//!
//! void *socket_worker(void* args) { /* Process socket data */ }
//!
//! pthread_t stdin_thr;
//! pthread_create(&stdin_thr, NULL, stdin_worker, NULL);
//!
//! pthread_t sock_thr;
//! pthread_create(&sock_thr, NULL, socket_worker, NULL);
//! ```
//!
//! Rust's async/.await combines the best of the two. It allows you to
//! write concurrent tasks in respective async blocks/functions and
//! execute them concurrently in threading style, but there is only
//! one thread involved. The thread switches from one task to another
//! whenever a task yields and the other task becomes ready. It
//! actualy does select() or epoll() under the skin. With
//! async/.await, if frees the programmers from the hassle of
//! hand-crafted state machines, but still maintains the benefit of
//! code readability and low overhead.
//!
//! ```rust
//! let stdin_task = async move {
//!     let line = stdin.read_line().await;
//! };
//! let socket_task = async move {
//!     let mut buf = vec![0u8; 4];
//!     sock.recv(&mut buf).await;
//! };
//! join!(stdin_task, socket_task);
//! ```

// Declare modules. Each module corresponds to a file. For example,
// `mod message` is for the `message.rs` file.
mod message;
mod utils;

// Imports the types and functions we want to use.
use crate::message::Action;
use crate::utils::recv_msg;
use clap::Parser;
use futures::try_join;
use message::Message;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UdpSocket;
use utils::send_msg;

// The argument type and return type of a function can help you guess
// the purpose of the function. Let's see the function for example.
//
// ```
// async fn recv_msg(sock: &mut UdpSocket) -> io::Result<Option<Message>>
// ```
//
// First, the sock type is a &mut reference, indicating that the
// function may change the state of the socket.
//
// The return type `io::Result<Message>` gives us much information.
//
// - `Result` indicates that the function may fail at some point.
// - `Message` is the pearl we want to get.

// The comment starting with `///` is a "doc comment", which is
// distinct from a normal `//` comment. The doc comment can be used to
// generate documents, while the normal comment is simply ignored by
// the compiler.
//
// The doc comment must be placed above a function, a struct or a
// field, etc.
/// An paper-scissor-stone game player example using async/.await.
#[derive(Debug, Clone, Parser)]
struct Args {
    /// The name of the player.
    pub name: String,

    /// The IP:port address that the player binds to. For example,
    /// "127.0.0.1:44444".
    pub self_addr: SocketAddr,

    /// The IP:port address of the opponent player. For example,
    /// "127.0.0.1:55555".
    pub other_addr: SocketAddr,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    // Read command line args.
    //
    // The parse() is given by clap::Parser trait, derived on the Args
    // struct. It automatically parses the command-line args according
    // to field types in Args.
    //
    // If a required argument is missing or it's unable to convert to
    // requested type, Args::parse() emits the help message and kill
    // the process.
    let opts = Args::parse();

    // It is an convenient way to unpack a struct. There's no need to
    // write `let name = opts.name;`, etc.
    let Args {
        name,
        self_addr,
        other_addr,
    } = opts;

    // Creates a UDP socket, providing the local and remote addresses.
    //
    // The .await marks the point where a thread can make a pause and
    // "yield" the execution. For example, the socket reading
    // `socket.recv().await` can yield when the data is not avaible,
    // and pauses until the data becomes ready.
    let sock = UdpSocket::bind(self_addr).await?;
    sock.connect(other_addr).await?;

    // Sleep for a while to wait for the oppoent to get ready.
    //
    // Note that we use tokio's sleep(), not std's sleep, because
    // std's sleep is blocking and we don't want it to block in the
    // async context.
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Calling an async function creates a pending unit to be
    // evaluated called "Future". The future should be .await to be
    // exectured and get the actual return value.
    //
    // Note that if a future is created but not called on .await, it
    // will not be executed.
    //
    // ``` async fn foo() -> u8 { ... }
    //
    // let future = foo();
    // let output = future.await;
    // ```

    // Let's send a hello to the opponent.
    //
    // `async { .. }` block creates a future in-place.  This
    // future evaluates to a Result when it is awaited.
    let say_hello_future = async {
        let msg = Message::Hello { name };
        let result: io::Result<()> = send_msg(&sock, msg).await;
        result
    };
    let result = say_hello_future.await; // Evaluate/Execute the future
    result?; // Return if error

    // Wait for opponent's hello message.
    //
    // Note that we do not use `async {}` block like one above.
    // Actiually, it was not needed here because the main function is
    // already async. We write the block for educatoinal purpose.
    //
    // The code can be shortened to:
    // ```
    // let Message::Hello {
    //     name: opponent_name,
    // } = recv_msg(&sock).await?
    // else {
    //     panic!("unexpected message type");
    // };
    // ```
    let opponent_name = {
        let result = recv_msg(&sock).await;

        match result {
            Ok(Message::Hello { name }) => name,
            Ok(_) => panic!("unexpected message type"),
            Err(err) => return Err(err),
        }
    };

    println!("{opponent_name} enters the game!");

    // Here creates two async tasks, one scanning user input from the
    // terminal, the other reading data from the socket.
    //
    // Both tasks share the UDP socket. Let's wrap the socket in the
    // `Arc` pointer, so that both tasks can get a copy of the pointer
    // to operate on the same socket.
    let sock_ptr1 = Arc::new(sock);
    let sock_ptr2 = sock_ptr1.clone(); // The .clone() copeis the pointer, not the underlying socket.

    // Now creates to futures. Note that we does not call .await on
    // purpose.
    let my_turn_future = my_turn(sock_ptr1);
    let opponents_turn_future = opponents_turn(sock_ptr2);

    // Let's execute both futures concurrently and returns both
    // outputs when both futures complete. The `try_join!` macro is
    // the sibling of `join!`. It similar to `join!` but checks if any
    // one of future evaluates to `Err()`.
    let (my_action_option, oppo_action) = try_join!(my_turn_future, opponents_turn_future)?;

    // Check if the user provides a move or quits by unpacking the
    // `Option`. There are two more equivalent ways to write the code.
    //
    // ```
    // let Some(my_action) = my_action_option else {
    //     return Ok(());
    // };
    // ```
    //
    // ```
    // let my_action = match my_action_option {
    //     Some(act) => act,
    //     None => return Ok(()),
    // };
    // ```
    let my_action = if let Some(act) = my_action_option {
        act
    } else {
        println!("You quits. Loser!");
        return Ok(());
    };

    // Print the moves of both sides.
    //
    // Here creates a `|args| { ... }` closure to that will be called
    // twice. It works like a function but is anonymous.
    let get_action_name = |action: Action| match action {
        Action::Rock => "rock",
        Action::Paper => "paper",
        Action::Scissor => "scissor",
    };
    println!("You plays {}.", get_action_name(my_action));
    println!("The opponent plays {}.", get_action_name(oppo_action));

    // Determine the winner.
    match (my_action, oppo_action) {
        (Action::Rock, Action::Rock)
        | (Action::Paper, Action::Paper)
        | (Action::Scissor, Action::Scissor) => println!("Fair."),

        (Action::Rock, Action::Scissor)
        | (Action::Paper, Action::Rock)
        | (Action::Scissor, Action::Paper) => println!("You win!"),

        (Action::Rock, Action::Paper)
        | (Action::Paper, Action::Scissor)
        | (Action::Scissor, Action::Rock) => println!("You lose!"),
    }

    Ok(())
}

/// Get my move from the terminal.
///
/// This function comes in three outcomes:
/// - `Ok(Some(action))` - The user gives an action.
/// - `Ok(None)` - The user quits during the process.
/// - `Err(err)` - An I/O error occurred.
async fn my_turn(sock: Arc<UdpSocket>) -> io::Result<Option<Action>> {
    // Create a Stdin object from tokio library.  We use tokio's
    // Stdin instead of standard library's because it supports
    // .await syntax.
    let stdin = tokio::io::stdin();

    // Wrap the stdin in tokio's BufReader to enable reading
    // line-by-line.
    let reader = BufReader::new(stdin);

    // Convert the reader to a stream of lines.
    let mut lines = reader.lines();

    // The loop repeats until a valid command is read from the user.
    // That is, whenever a valid command is recognized, it immediately
    // break the loop.
    let action: Action = loop {
        println!("Enter your move and press enter.");
        println!("- r: Rock");
        println!("- p: Paper");
        println!("- s: Scissor");
        println!("- q: Quit");

        // Wait for the next line. It returns a result.
        let result: Result<_, _> = lines.next_line().await;

        // Unpack the result. It gets an Option<String>. The `?`
        // syntax unpacks a `Result` variable. It unpacks the inner
        // value if the variable is `Ok`. Otherwise, it returns an
        // error from the function. The syntax is valid only when the
        // return type of current function is also `Result`.
        let opt: Option<String> = result?;

        // Unpack the opt Option<String>.
        //
        // If it is Some(line), get the inner value. Otherwise, it
        // reaches the end of file so we return early.
        let line: String = match opt {
            Some(line) => line,
            None => return Ok(None),
        };

        // The code above can be shortened to the following. We wrote
        // the verbose version for clarity.
        //
        // ```
        // let Some(line) = lines.next_line().await.unwrap() else {
        //     break;
        // };
        // ```

        // Parse the input line.
        let action: Action = match line.as_str() {
            "p" => Action::Paper,
            "s" => Action::Scissor,
            "r" => Action::Rock,
            "q" => {
                // User requests quit. Let's return early.
                return Ok(None);
            }
            _ => {
                // In this hand, user gives a command not understood
                // by us. Re-run the loop to get the next line.
                println!("Command not understood");
                continue;
            }
        };

        // Exit the loop.
        break action;
    };

    // Send a message to the opponent.
    let msg = Message::Act(action);
    send_msg(&sock, msg).await?;

    // The last `Ok` is necessary because the function expects a
    // `Result<_>` return value.
    Ok(Some(action))
}

/// Gets the opponent's move by reading the socket.
async fn opponents_turn(sock: Arc<UdpSocket>) -> io::Result<Action> {
    // Receive a message from the opponent
    let msg = recv_msg(&sock).await?;

    // Unpack a message.
    let Message::Act(action) = msg else {
        panic!("Unexpected message type");
    };

    Ok(action)
}
