# Cosmic Garden

Cosmic Garden is a spiritual successor to classic MUD engines,
but beefed up and reimagined with modern concurrency, etc.
It’s not *just* a MUD engine; it’s an **OMFG** engine:

* *OMFG* — *Original MUD Features Gussied‑up*.

Built from scratch in Rust, Cosmic Garden heavily embraces an event‑driven design:

* multi‑threaded and parallel by default
* scheduler / tick‑drive symbiosis
* futures, weak/hard references, broadcast/mpsc …
* no polling, no legacy baggage

A modern engine with the soul of a MUD and the posture of a cosmic gardener,
and thus more an OMFG than "mere" MUD engine.

## Technobabble

…CG drives the World by default at 100Hz, so hold on to your britches and hats.

The major threads in a nutshell:

* main() just bootstraps the world, kickstarts the other threads and then sits
  listening for a) incoming connections, b) for signal from Janitor to shut the
  curtains.

  * each incoming client becomes their own independent `tokio::spawn`.
* Librarian handles bootstrapping of help files, entity database, and item
  blueprints, and so on and so forth. Librarian also deals with new entries
  and item spawning.
* Life-thread ticks the World, runs combat, deals with transportation
  requests, entity spawning, etc.

  * a persistent co-worker deals with combat reporting in stead of life-thread
    itself.
  
  In case things get hairy, life-thread spawns extra workers as needed, just
  like Janitor.
* Janitor acts as a… janitor. He and his co-spawns are the only ones touching
  disk I/O directly past initial bootstraps.

### Security

`Argon2` and `HIBP` - 'nuff said?

### Genesis

CG does not rely on configs or 3rd party anything. It can and will generate
a skeleton but fully functional world from scratch if none is present
yet.
