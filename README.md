# Project CO2

## Getting Started

1. [Install Rust](https://rustup.rs)
2. Install the target platform 
   1. `rustup target add thumbv7m-none-eabi`
3. Install `cargo-embed`
   1. `cargo install cargo-embed`
4. Navigate to the `project-co2`` directory
   1. `cd project-co2`
5. Ensure the STLinkV2 is connected to the Blue Pill board and plugged in, and build + flash with the command
   1. `cargo embed`

If successful, you should see something like the below in the terminal and the green LED should be blinking 4 times per second

```
        WARN probe_rs::config::target > Using custom sequence for ARMv7 STM32F101C8
     Erasing sectors ✔ [00:00:00] [###########################################################] 11.00 KiB/11.00 KiB @ 29.47 KiB/s (eta 0s )
 Programming pages   ✔ [00:00:00] [###########################################################] 11.00 KiB/11.00 KiB @ 21.92 KiB/s (eta 0s )    Finished flashing in 0.891s
        Done processing config default
```