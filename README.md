# Usage

This library defines a macro / trait for populating an SoA from a json blob.
Even though it uses slow, serialization or whatever, the key insight is this:

This gets run at load time, to populate a SoA that holds 1 prototype per entity.

Then for spawning at runtime you copy prototype 'i' into the runtime SoA