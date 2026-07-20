# packet-gen-wasm

This is a WASM module for [Vape Rewrite] that supplies data for creating dummy enums/messages.

## The Problem

Miniblox uses latest Vite (you can see it uses Rolldown).
Miniblox also has chunking enabled, this helps a lot for getting things we need references to.
However, the bundle only exports 13 packets... For context, there's ~115 packets in the bundle total, so 102 packets are missing.
So... How do we get the missing packets?

The way we do it is...

## The Solution

We just automatically derive the data from the packets we need using this WASM Module.
All we need to generate dummy messages is the fields list, packet name, and runtime, and enums just needs the enum name and entries.
So, that's what this does. You just give it the packet name and runtime, and it emits the fields list or enum entries for you.
If you want to see this in depth, see [Vape Rewrite].


[Vape Rewrite]: https://codeberg.org/Miniblox/VapeRewrite
