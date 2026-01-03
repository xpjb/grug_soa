# Usage
This library is for defining game entity prototypes in JSON and loading them into arrays at runtime.

This library defines a macro / trait for populating an SoA from a json blob.
Even though it uses slow, serialization or whatever, the key insight is this:

This gets run at load time, to populate a SoA that holds 1 prototype per entity.

Then for spawning at runtime you copy prototype 'i' into the runtime SoA

## Todo
probably more error handling etc, probably shouldnt crash the program if the user made a bad prototype but it should be a warning the user can handle

Types should be Clone, Default, and Deserialize

maybe the macro can contain specific asserts to that affect or see what kind of errors it produces when you do the wrong thing, if they are very intelligible or not.

Also stuff about definitely remembering which thing was which so it can be re loaded deterministically. maybe a dedicated name or uuid field - u64 random number


## Counter points
What if it had to spawn ones of another one, or contain references?
Maybe you enter them yourself after