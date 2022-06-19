# Sun Gold Miner

A small website designed to save cumulative solar panel data on power generation and display in a nice manner.

## Background

My family house just had solar panels installed, and we wanted to help the community in our complex also adopt
a more environmental approach to power generation; my father and I figured it'd be easier if we created a tracker
to show our power consumption and savings thanks to the solar panels it'd help convince our fellow neighbors
to do the same.

I named the tool "Sun Gold Miner" cause it datamines solar panel data. Cheesy, I know.

## Implementation

Our solar panel setup has a three-phased power analyzer (a Carlo Gavazzi WM15) that keeps track of voltages consumed
and produced by our house's electric setup. The analyzer uses a weird TCP/IP protocol called Modbus, which effectivelly
allows for a weird reading of registers on the control interface. Thankfully, there's a Modbus crate for Rust, and I figured
it's a good way to practice my usage of Rust.

Effectively, we're building a web-based backend in front of the power analyzer in order to better
query for the information we want to display, whilst simultaenously making a good-looking and simple
interface for it.

The backend is written in Rust, using the Rocket web framework and the [modbus](https://crates.io/crates/modbus) crate.
The frontend is written using Next.js and Tailwind CSS.