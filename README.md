# Rust async/.await Starter: Rock-Paper-Scissor Game

This project provies a rock-paper-scissor client example to
demonstrate Rust's async/.await feature.

The code is written in a more readable way along with line-by-line
comments. The purpose is to help newcomers to understand the usage of
async/.await syntax and learn concurrent programming.

The implemented program connects to the opponent via an UDP socket,
reads user's and opponent's moves simultanously, and determines the
winner.

## Get Started

Install the Rust toolchain from [rustup.rs](https://rustup.rs/).

```sh
# Compile
cargo build

# On Alice's computer
./target/debug/async-game-example Alice 127.0.0.1:4444 127.0.0.1:5555

# On Bob's computer
./target/debug/async-game-example Bob 127.0.0.1:5555 127.0.0.1:4444
```

## Why Using async/.await?

The step to read both players' moves simultanously is the place
eleases the power of Rust. It has to wait for stdin input and socket
data in the mean time. The challenging part is that if the program
solely blocks on the stdin input, it may misses the message from the
remote user.

In in old days of C/C++, we use select() to poll on the stdin and
socket and wait until one of them to become ready. However, the time
selects() saying stdin is ready does not mean that the input is
complete. The program may read an incomplete message from the user. It
has to store the state of the input and select() again to wait for the
remaining part. It runs into the hassle to write state machine to keep
track of the the local and remote user input states.

```c
while (1) {
    fd_set rfds;
    FD_SET(0, &rfds);     // stdin
    FD_SET(sockfd, &rfds);  // socket

    int ret = select(2, &rfds, NULL, NULL, NULL);
    if (ret < 0) { exit(1); }

    if (FD_ISSET(0, &rfds)) {
        // Process stdin input
    } else if (FD_ISSET(sockfd, &rfds)) {
        // Process socket data
    }
}
```

One common alternative is to use threads. We can spawn two threads,
one for the local stdin and the other for the socket. This way
simplifies the programming of concurrent operations. However, it comes
with costs. Spawning threads introduces overheads, and on some
embedded systems, threading is not supported.

```c
void *stdin_worker(void* args) { /* Process stdin input */ }

void *socket_worker(void* args) { /* Process socket data */ }

pthread_t stdin_thr;
pthread_create(&stdin_thr, NULL, stdin_worker, NULL);

pthread_t sock_thr;
pthread_create(&sock_thr, NULL, socket_worker, NULL);
```

Rust's async/.await combines the best of the two. It allows you to
write concurrent tasks in respective async blocks/functions and
execute them concurrently in threading style, but there is only one
thread involved. The thread switches from one task to another whenever
a task yields and the other task becomes ready. It actualy does
select() or epoll() under the skin. With async/.await, if frees the
programmers from the hassle of hand-crafted state machines, but still
maintains the benefit of code readability and low overhead.

```rust
let stdin_task = async move {
    let line = stdin.read_line().await;
};
let socket_task = async move {
    let mut buf = vec![0u8; 4];
    sock.recv(&mut buf).await;
};
join!(stdin_task, socket_task);
```

## License

This software is distributed under MIT license. Check the
[LICENSE.txt](LICENSE.txt) to see the full license text.

By using or referring this software, please note the source
_NEWSLAB, Depart. of CSIE, National Taiwan University_. Thanks.
