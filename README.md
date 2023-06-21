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

 ## Planning ..

 This repository provides a shared playground in which the entire team can experiment with and refine ideas before we commit resources to them, so we should keep the following mantra in mind:

> The backend sets the constraints, but the frontend sets the requirements.

As we begin to replace legacy components, we shouldn't limit ourselves exclusively to drop-in replacements. Instead, find out where the old ones went wrong and how we can make life easier (and thus more maintainable) for those making use of the features!