#![deny(missing_docs)]
/*!
# stacks-signer-api: an API for configuring and interacting with a Stacks signer.

This library contains API calls to configure a signer to auto-sign transactions or manually sign a specific transaction upon request.

Usage documentation can be found in the [README](https://github.com/Trust-Machines/core-eng/stacks-signer-api/README.md).
*/

/// Sqlite database
pub mod db;
/// Delegator key
pub mod key;
/// Signer API Routes
pub mod routes;
/// Signer Interactions
pub mod signer;
