# Philosophy

This section collects the more detailed discussions around the design of
Trible Space and the reasoning behind certain choices. It is meant as an
optional read for the curious.

We prioritise a simple and predictable system over clever heuristics. Each
component should be understandable on its own and interact cleanly with the
rest of the stack.

Developer experience is equally important. APIs aim to be straightforward and
use synchronous building blocks that can be composed as needed.

Finally, we strive for soundness and performance. Safety checks guard against
invalid data while efficient data structures keep the core fast.
