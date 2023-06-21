# thundercell

Internal repository to assist the backend team with discovery, prototyping and documenting future
efforts to replace aging components.

Feel free to use namespaced directories within this project to carry out PoC / scoping work.
We will soon use issues and projects for meta-tracking before migrating approved works back
to Bugzilla (in a fully fleshed form)

Here, we forge only the strongest thunder.

## Oxidization

Where possible, we should look to Rust for replacement components to ensure memory safety and performance in the backend. This is especially important for critical components such as:

 - protocols
 - databases
 - OS/system integration