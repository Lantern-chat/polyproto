// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Display;

use spki::{AlgorithmIdentifierOwned, SignatureBitStringEncoding};

/// A signature value, generated using a [`SignatureAlgorithm`]
pub trait Signature: Display + PartialEq + Eq + SignatureBitStringEncoding + Clone {
    /// The underlying signature type
    type Signature;
    /// The signature value
    fn as_signature(&self) -> &Self::Signature;
    /// The [`AlgorithmIdentifierOwned`] associated with this signature
    fn algorithm_identifier() -> AlgorithmIdentifierOwned;
    /// From a byte slice, create a new [Self]
    fn from_bytes(signature: &[u8]) -> Self;
}
