This is my attempt (but I stalled!) to implement the embeddable JVM in Rust + LLVM where the goals include 

1. Make the interop Rust<>JVM as easy as possible, 
2. Sandbox by default - all system calls are intercepted at Rust layer (implementing java/lang/system, etc),
3. Lightweight startup,
4. (Optional) Native image compilation.

Unfortunately, as life gets busy and my passion fades, I have to put this project on hold. I hope to come back to it one day.

For those curious, all the currently-working Java programs are located in [tests/cases](./tests/cases) directory.