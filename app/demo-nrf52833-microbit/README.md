# micro:bit v2.21 demo application

This file is a log containing the history of the porting of Hubris
kernel to the micro:bit v2.21 board and to other versions, possibly.

It will be written in historical order, layed out as a real paper log,
so that the reader can enjoy my descent into madness as it unfolds.

## 2025-08-23

I knew Hubris was cool, but this is amazing!

In just 4-5 hours I managed to run Hubris on a new micro:bit. The
kernel consists of just three tasks
* `jefe`, which is the supervisor
* `jiffy`, which is the interpreter of the Hubris/Humility Interchange
  Format (which I understand being the format used to debug
  applications running on a board), and
* `idle`, which is Hubris' version of the idle task

The total amount of reading I've done was
* [Hubris Reference](https://hubris.oxide.computer/reference/), which,
  to be fair, I've read multiple times over the years as one would
  read narrative, so it's not calculated in the 4-5 hours stated about
* [Hubris on PineTime](https://artemis.sh/2022/03/28/oxide-hubris-on-pinetime.html),
  which is a very nice log of the porting to a similar device from nordicsemi
* [nRF52833 data sheet](https://docs.nordicsemi.com/bundle/ps_nrf52833/page/keyfeatures_html5.html),
  which I can't tell whether is well written or not being the first
  time I read such a document, but was nonetheless interesting

The net result of the first day was a running kernel on top of the
micro:bit that does basically nothing, but allowed me to get
acquainted with the build process as well as `humility`.

### Understanding memory layout

Given I've not written any custom code yet, the main difficulty I had
so far was understanding "what goes where" in terms of memory layout.

For example, the [official data sheet](https://docs.nordicsemi.com/bundle/ps_nrf52833/page/memory.html)
reports that memory range from `0x00000000` to `0x20000000` is for
"code", but only memory from `0x00000000` to `0x00800000` is actually
flash memory. Moreover, it mentions multiple ranges as "RAM" and
"SRAM", which are yet again different from "Data RAM" and "Code
RAM". These might seem obvious to those skilled in embedded
programming, but for me this is the first serious attempt at building
and running a more complex application on a board, so I had to tinker
a bit to get them right.

Hubris build process wants exact ranges written under
`chips/<your-chip>/memory.toml`, and wants at least `flash` and `ram`
entries.

Interestingly, I ran into the difference between `ram` and `sram` by
copy-pasting the wrong bit from another file, so I only had `sram` and
no `ram` entry. The error I got back from `cargo xtask dist` was a bit
surprising, stating that `STACK` was not defined in the
`target/demo-nrf52833-microbit/dist/link.x`.

```
$ cargo xtask dist app/demo-nrf52833-microbit/app.toml
...
target/thumbv7em-none-eabihf/release/task-idle -> target/demo-nrf52833-microbit/dist/idle.elf
rust-lld: error: link.x:9: memory region not defined: STACK
>>>   PROVIDE(_stack_start = ORIGIN(STACK) + LENGTH(STACK));
>>>                                      ^
Error: command failed, see output for details
```

I had a hunch that this was related to something about the build
process not being able to understand which was the right memory range,
so I ran the equivalent `dist` command for the `stm32f4-discovery`
board and then had a good look at the resulting `link.x` and
`memory.x` files. It was then easy to spot the problem.

```
$ cat target/demo-stm32f3-discovery/dist/memory.x
MEMORY
{
FLASH (rwx) : ORIGIN = 0x08006800, LENGTH = 0x00000060
STACK (rwx) : ORIGIN = 0x20003000, LENGTH = 0x00000100
RAM (rwx) : ORIGIN = 0x20003100, LENGTH = 0x00000000
}
__this_image = 0x08000000;
__IMAGE_DEFAULT_BASE = 0x08000000;
__IMAGE_DEFAULT_END = 0x08040000;
SECTIONS {
} INSERT AFTER .uninit

$ cat target/demo-nrf52833-microbit/dist/memory.x
MEMORY
{
FLASH (rwx) : ORIGIN = 0x00000000, LENGTH = 0x00080000
SRAM (rwx) : ORIGIN = 0x20000000, LENGTH = 0x00020000
}
__this_image = 0x00000000;
__IMAGE_DEFAULT_BASE = 0x00000000;
__IMAGE_DEFAULT_END = 0x00080000;
SECTIONS {
} INSERT AFTER .uninit
```

As one can easily spot, the first one had `FLASH`, `STACK` and `RAM`,
while my one had `FLASH` and `SRAM` only. Simply renaming `sram` to
`ram` did the trick.

### Brief `probe-rs` detour

Another project I find interesting is `probe-rs`, so I took the chance
to give it a try. It actually helped me figuring out the correct value
to put in `boards/nrf52833-microbit.toml`!

The board version I'm using for this experiment is `v2.21`, which,
according to the data sheet has

> ... 512 kB of flash memory and 128 kB of RAM ...

(Note: I kinda like the fact that nordicsemi folks just don't use
`kiB` and keep using the good'ol `kB`)

This information is "carved in stone" in FICR, acronym for "Factory
information configuration registers". The [data sheet page](https://docs.nordicsemi.com/bundle/ps_nrf52833/page/ficr.html#topic)
related to it says it starts at `0x10000000` and contains interesting
stuff, like memory page size and number of pages. Running the
following confirmed a bunch of values, reassuring me that my purchase
was not a scam

```
$ probe-rs read --chip nRF52833_xxAA b32 0x10000010 2
00001000 00000080
$ probe-rs read --chip nRF52833_xxAA b32 0x1000010C 1
00000080
$ probe-rs read --chip nRF52833_xxAA b32 0x10000110 1
00000200
```

Specifically, the value of `INFO.RAM` can be read at offset `0x10C`,
which is the middle row above. The actual value for my board is `0x80`
representing the 128 kB of RAM as per [this table](https://docs.nordicsemi.com/bundle/ps_nrf52833/page/ficr.html#register.INFO.RAM).

Value at offset `0x110` is `INFO.FLASH`, which is the equivalent
information but for flash memory (see [table here](https://docs.nordicsemi.com/bundle/ps_nrf52833/page/ficr.html#register.INFO.FLASH)).

Finally, values at `0x010` and `0x014` are the previously mentioned
memory page size and total number of pages.

### Building and running

Once I got the chip, board, and app toml files right, I was able to
get compilation errors for my `main.rs` file. Yay!

It's been a while since the last time I built something in Rust, and,
again, I have very little experience with embedded programming, but I
remembered from recordings of the [OSFC](https://www.osfc.io/) that
embedded Rust developers usually split their crates into PAC and
HAL... If only I remembered what they were for!

Yet again, existing code was my friend, and I had a look at the code
for STM32 Discovery board, tracked down its dependencies, figured it
was using the PAC crate, not the HAL, remembered that PAC stands for
"Peripheral Access Crate" while HAL stands for "Hardware Abstraction
Layer", and finally found the project implementing PAC crate for
nRF52833 among others, namely [nrf-pacs](https://github.com/nrf-rs/nrf-pacs).

I believe this information could also be found in the fork of Hubris
containing the PineTime port, but it dates back to 2022, and was not
kept up to date with changes to the structure of Hubris codebase, so I
had to find the right crate for myself. Also, I was already arms-deep
into code, I just felt like I had to keep doing it!

Here I ran into another weird error message.

My initial thought was that I would not have written any new code, so
it could have been possible to run Hubris _without_ adding the
`nrf52833-pac` crate. It turns out you need that create to obtain a
functioning `device.x` file. Here's the error

```
ERROR(cortex-m-rt): The interrupt vectors are missing.
```

Once again, I was saved by STM32 code containing this comment.

```rust
// We have to do this if we don't otherwise use it to ensure its vector table
// gets linked in.
#[cfg(feature = "stm32f3")]
extern crate stm32f3;
#[cfg(feature = "stm32f4")]
extern crate stm32f4;
```

After doing my homework, I managed to fully build the kernel, but I
still thought it was too good to be true, so I flashed my micro:bit

```
$ cargo xtask flash app/demo-nrf52833-microbit/app.toml
...
building crate demo-nrf52833-microbit
    Finished `release` profile [optimized + debuginfo] target(s) in 0.11s
target/thumbv7em-none-eabihf/release/demo-nrf52833-microbit -> target/demo-nrf52833-microbit/dist/kernel
flash   = 0x00000000..0x00080000
ram     = 0x20000000..0x20020000
Used:
  flash:   0x7000 (5%)
  ram:     0x8000 (25%)
humility: attaching with chip set to "nRF52833_xxAA"
humility: attached via CMSIS-DAP
humility: flash/archive mismatch; reflashing
humility: flashing done
```

I finally had the chance to run `humility`!

```
$ cargo xtask humility app/demo-nrf52833-microbit/app.toml -- tasks -ls
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.14s
     Running `target/debug/xtask humility app/demo-nrf52833-microbit/app.toml -- tasks -ls`
humility: attached via CMSIS-DAP
system time = 453977
ID TASK                       GEN PRI STATE
 0 jefe                         0   0 recv, notif: fault timer(T+23)
   stack unwind failed: Do not have unwind info for the given address.
 1 hiffy                        0   3 notif: bit31(T+84)
   stack unwind failed: Do not have unwind info for the given address.
 2 idle                         0   5 RUNNING
   |
   +--->  0x20000d00 0x00004e74 main
                     @ /hubris/task/idle/src/main.rs:28:13
```

All of a sudden, I felt like Gene Wilder in Frankenstein Junior!

### Closing thoughts

I spent the last few years reading books about FreeBSD and Minix, but
I always felt like those projects were very much beyond my reach in
terms of necessary skills to even build them, let alone port them to a
new board. Being able to do it with Hubris feels empowering!

Next step is writing tasks to control the board's LEDs so that I can
then port the very simple tutorial programs from the micro:bit
foundation to Hubris.


## 2025-08-31

Today I tried porting blinky, the standard "Hello World" program for
embedded systems, to the micro:bit board running Hubris.

During the time between this and the previous entry I ran into a very
nice youtube channel named "The Rusty Bits" and watched a video
explaining how to blink an led starting with raw pointers and then
improving the code to use PAC and eventually HAL, with the final
"bonus" section explaining the use of the less common Board Support
Package (BSP). [Here](https://www.youtube.com/watch?v=A9wvA_S6m7Y) is
the video.

My first thought was to just try copy/pasting the implementation using
raw pointers which is ugly, but easy to follow, and does not require
additional dependencies, although I acknowledge that I am already
importing the `nrf52833-pac` at this point.

### Bathing in unsafe Rust

At this point I knew I had to write a new task, and I decided I would
start with just raw pointers and volatile writes, so I copied the
`idle` task and made some changes.

The resulting code is committed and I won't reproduce it here, but
it's worth mentioning that I explicitly decided to NOT touch the
chip's `memory.toml` file, expecting to run into some memory
fault.

I was please to indeed run into a memory fault!

```
cargo xtask humility app/demo-nrf52833-microbit/app.toml -- tasks -ls
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.13s
     Running `target/debug/xtask humility app/demo-nrf52833-microbit/app.toml -- tasks -ls`
humility: attached via CMSIS-DAP
system time = 5075303
ID TASK                       GEN PRI STATE
 0 jefe                         0   0 recv, notif: fault timer(T+19)
   stack unwind failed: Do not have unwind info for the given address.
 1 hiffy                        0   3 notif: bit31(T+170)
   stack unwind failed: Do not have unwind info for the given address.
 2 idle                         0   5 RUNNING
   |
   +--->  0x20000d00 0x00004e74 main
                     @ /hubris/task/idle/src/main.rs:28:13

 3 leds                    103577   4 FAULT: mem fault (precise: 0x50000754) in task code (was: ready)
   |
   +--->  0x20000e00 0x00004edc core::ptr::write_volatile
                     @ /rustc/0d9592026226f5a667a0da60c13b955e0b486a07/library/core/src/ptr/mod.rs:2180:9
          0x20000e00 0x00004edc main
                     @ /hubris/task/microbit-leds/src/main.rs:24:6
```

Specifically, line 24 of the task is a `core::ptr::volatile_write` at
address `0x50000754`.

I expected I had to modify the `memory.toml` file to configure access
to the slice of memory dedicated to at least leds.

I had no idea what `dma = true` meant, so I only added RW permissions.
Again, by looking at the `stm32` demo application I noticed the "link"
between the `memory.toml` file and the `app.toml` one in the form of
`extern-regions`, which defines which task has access to which memory
region, at least in my understanding. This also clarified that, apart
from the `flash` and `ram` "magic words", all the others were
arbitrary, allowing me to define a new memory region for a little over
4 kB, only what was necessary to be able to read and write from the
two memory locations needed to make the led on the upper left corner
blink.

```toml
[[leds]]
address = 0x50000000
size = 0x1000
read = true
write = true
execute = false
```

I was amazed to see that the led turned on... but did not turn
of. I've been generous with memory range to which I granted write
access to the new task and did not expect this to be a memory fault,
but I wanted to verify it

```
$ cargo xtask humility app/demo-nrf52833-microbit/app.toml -- tasks -ls leds
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.11s
     Running `target/debug/xtask humility app/demo-nrf52833-microbit/app.toml -- tasks -ls leds`
humility: attached via CMSIS-DAP
system time = 52401777
ID TASK                       GEN PRI STATE
 3 leds                         0   4 RUNNING
   |
   +--->  0x20000e00 0x00004ef0 <core::ops::range::Range<T> as core::iter::range::RangeIteratorImpl>::spec_next
                     @ /rustc/0d9592026226f5a667a0da60c13b955e0b486a07/library/core/src/iter/range.rs:765:12
          0x20000e00 0x00004ef0 core::iter::range::<impl core::iter::traits::iterator::Iterator for core::ops::range::Range<A>>::next
                     @ /rustc/0d9592026226f5a667a0da60c13b955e0b486a07/library/core/src/iter/range.rs:850:14
          0x20000e00 0x00004ef0 main
                     @ /hubris/task/microbit-leds/src/main.rs:37:15
```

The program was running without any memory issue, and it is so
straightforward that the problem could not be any sort of
memory-related poltergeist, so I bumped the counter of the `nop` loop
from `400_000` to `100_000`. It turns out the led was indeed blinking,
but the initial speed was way too high, and decreasing it made the
blinking barely visible.

This is an interesting difference from the video I was following from
The Rusty Bits where the led was blinking at roughly 1 Hz. In my
previous session, I set the tick divisor passed to `start_kernel` to
an arbitrary 8000 and reducing it to 500 changed the speed at which
the led was blinking, which was reassuring, but it's pretty much clear
that the issue is the busy loop. Also, the advertised CPU frequency
for the nrf52833 is 64 MHz.

Next step is trying to get the led to blink at one actual Hertz or as
close as possible to it.

## 2025-09-01

To the best of my knowledge, Hubris is a micro-kernel inspired by
MINIX, so the natural follow up would be trying out the task
communication primitives.

I liked the idea of having a task controlling the leds only, so I did
not touch the code written so far and focused instead on writing a
task having access to a timer. It was time to use a PAC.

### (Not so) Brief detour to `nrf-pacs`

Blinking an LED by dereferencing pointers was simple enough, but I was
not expecting to be able to do much more work without any abstraction,
so I went back to read about `nrf-pacs`. The project seemed simple
enough so I checked out the source and read its `Cargo.toml` file and
the `xtask` code.

There are three `cargo xtask` subcommands, `generate`, `build`, and
`publish`, all of them fairly self explaining given that PACs are
generated from SVD files. The `generate` command uses `svd2rust` to
transform SVD files into Rust code. I'm not extremely familiar with
SVD files, but, thanks again to The Rusty Bits, I learned that it's a
standard format defined by ARM used to specify the various details
pertaining a board. The second one, `build` builds the generated
sources, ensuring the rest of the configuration (e.g. the target
architecture) makes sense. The last one publishes to
[crates.io](https://crates.io).

Overall, the project seems a bit stagnating, and despite having very
few dependencies, some of them could use an update.

As it's the case for most other PAC repositories, there's an `svds/`
folder containing a collection of huge XML files provided by
manufacturers. These files, as one might expect, must be updated
regularly, which is a manual process, and the last few updates were
targeted to specific boards.

I googled the latest SVD files and managed to find a single zip file
with all of them, so I blindly replaced the contents of the `svds`
files with the downloaded ones. After fixing newlines a bit, I tried
building them and ran into an issue with the access specification
being mispelled `read-writeonce` instead of `read-writeOnce`, which
you can read more about
[here](https://github.com/rust-embedded/svd2rust/issues/91#issuecomment-303558806)
and was an easy fix.

SVDs were up to date.

After that I checked the few dependencies and bumped only `cortex-m`
by a couple minors and, most importantly, `svd2rust` from `0.25.1` to
`0.37.0`, which was a big jump! Yet, after doing that the whole build
process worked with some warnings about `critical-section` feature not
being defined,

I only had to give it a try on my micro:bit before opening a pull
request.
