## User State contract

The user state contract holds all of the user's state, including:
- Abstraction rules of all types. Future types are supported without migration by using `Rule::Custom` and a binary message with can be serialized/deserialized elsewhere. Abstraction logic is not applied here; rules are intepreted by gatekeeper logic contracts.
- The time of the user’s last activity (owner activity). This is used to calculate whether inheritance dormancy period (`cooldown`) has passed or not when checking messages against an inheritance `AbstractionRule`.

User state contract logic is generally not designed to be migrated (updated); when a new user account is created, an existing user state can be attached to it, allowing abstraction rules from an existing user to be preserved when all other logic contracts are upgraded.