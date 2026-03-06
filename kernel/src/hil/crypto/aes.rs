// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2026.

//! Interface for symmetric encryption and decryption with AES
//!
//! This module defines the [`Aes`] HIL trait along with several helper types which facilitate
//! symmetric encryption and decryption operations in accordance with the Advanced Encryption
//! Standard defined by [FIPS 197].
//!
//! [FIPS 197]: https://csrc.nist.gov/pubs/fips/197/final
//!
//! This interface supports both unauthenticated modes as well as authenticated (AEAD) modes.

use crate::ErrorCode;

/// Standard AES block size, in bytes
pub const BLOCK_SIZE: usize = 16;

/// Encryption key length
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyLength {
    Aes128,
    Aes192,
    Aes256,
}

impl KeyLength {
    /// Returns length of the key, in bytes
    pub const fn bytes(&self) -> usize {
        match self {
            KeyLength::Aes128 => 16,
            KeyLength::Aes192 => 24,
            KeyLength::Aes256 => 32,
        }
    }
}

/// AES-GCM tag length
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagLength {
    Tag32,
    Tag64,
    Tag96,
    Tag104,
    Tag112,
    Tag120,
    Tag128,
}

impl TagLength {
    /// Returns length of the tag, in bytes
    pub const fn bytes(&self) -> usize {
        match self {
            TagLength::Tag32 => 4,
            TagLength::Tag64 => 8,
            TagLength::Tag96 => 12,
            TagLength::Tag104 => 13,
            TagLength::Tag112 => 14,
            TagLength::Tag120 => 15,
            TagLength::Tag128 => 16,
        }
    }
}

/// Block cipher mode
///
/// This enum represents various block cipher modes that may be supported by an AES HIL
/// implementation. Each variant documents the expected sequence of [`AesClient`] callbacks and
/// their semantics when operating in that mode.
///
/// The formal definitions of these modes can be found in the following NIST publications:
///
/// * [NIST SP 800-38A](https://csrc.nist.gov/pubs/sp/800/38/a/final)
/// * [NIST SP 800-38C](https://csrc.nist.gov/pubs/sp/800/38/c/upd1/final)
/// * [NIST SP 800-38D](https://csrc.nist.gov/pubs/sp/800/38/d/final)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Cipher Block Chaining mode, as specified in NIST SP 800-38A, section 6.2
    ///
    /// Expected callbacks:
    ///
    /// * `read_key()` to read the AES key.
    /// * `read_iv()` to read an IV of exactly [`BLOCK_SIZE`] bytes.
    /// * `read_input()` to read the input data (plaintext for encryption, ciphertext for
    ///   decryption). Input data must always be a multiple of the [`BLOCK_SIZE`].
    /// * `write_output()` to write the output data (ciphertext for encryption, plaintext for
    ///   decryption). Output data is always the same length as input data.
    /// * `write_iv()` to write the last ciphertext block, which can be used as the IV for the next
    ///   CBC operation. This allows easy chaining of incremental CBC operations.
    Cbc,

    /// Counter with Cipher Block Chaining-Message Authentication Code mode, as specified in NIST
    /// SP 800-38C
    ///
    /// Expected callbacks:
    ///
    /// * `read_key()` to read the AES key.
    /// * `read_nonce()` to read the a nonce.
    /// * `read_input()` to read the input data (plaintext for encryption, ciphertext followed by
    ///   an authentication tag for decryption). Input data _need not_ be a multiple of the
    ///   [`BLOCK_SIZE`].
    /// * `read_ad()` to read associated data.
    /// * `write_output()` to write the output data (ciphertext followed by an authentication tag
    ///   for encryption, plaintext for decryption).
    ///
    /// The formatting and counter generation functions used by the underlying driver/hardware are
    /// not specified by this HIL, however in practice most implementations are likely to use the
    /// example functions specified in Appendix A of NIST SP 800-38C (for compatibility with IEEE
    /// 802.11 networking).
    ///
    /// Assuming the use of the aforementioned example functions:
    ///
    /// * The client may elect to provide a nonce of any size between 7 and 13 bytes, inclusive. The
    ///   driver shall return [`ErrorCode::INVAL`] if a nonce of any other length is provided.
    /// * The maximum amount of data that may be encrypted is _inversely proportional_ to the size
    ///   of the nonce. The larger the nonce, the smaller the maximum payload, but the more times
    ///   CCM may be invoked before exhausting the nonce space.
    ///
    /// Limitations and tradeoffs of nonce selection may differ if the driver/hardware implements
    /// different formatting and counter generation functions. However, the driver shall guarantee
    /// to abort and return [`ErrorCode::INVAL`] or [`ErrorCode::SIZE`] as appropriate if the client
    /// provides inputs that fall outside the parameters of its implementation.
    Ccm {
        /// Length of associated data, in bytes
        ad_len: usize,
    },

    /// Counter mode, as specified in NIST SP 800-38A, section 6.5
    ///
    /// Expected callbacks:
    ///
    /// * `read_key()` to read the AES key.
    /// * `read_ctr()` to read the counter value for the first block.
    /// * `read_nonce()` to read the fixed nonce value, but only if `read_ctr()` returns fewer than
    ///   `BLOCK_SIZE` bytes.
    /// * `read_input()` to read the input data (plaintext for encryption, ciphertext for
    ///   decryption). Input data _need not_ be a multiple of the [`BLOCK_SIZE`].
    /// * `write_output()` to write the output data (ciphertext for encryption, plaintext for
    ///   decryption). Output data is always the same length as input data.
    ///
    /// Counter blocks are constructed by concatenating a nonce with an incrementing counter value
    /// retrieved from the client via `read_nonce()` and `read_ctr()`, respectively. The nonce
    /// occupies the high order bytes, and the counter in the low order bytes. The counter is
    /// incremented for each block using the "standard incrementing function" as defined in appendix
    /// B.1 of the NIST spec, while the nonce remains fixed.
    ///
    /// The client may elect to provide a counter of any size between 1 and `BLOCK_SIZE` bytes,
    /// inclusive.
    ///
    /// As an additional, Tock-specific constraint, the driver shall **not** allow the counter to
    /// overflow or wrap. If a client requests an operation that would overflow or wrap the counter,
    /// the driver shall abort the operation and return [`ErrorCode::SIZE`].
    Ctr,

    /// Electronic Codebook mode, as specified in NIST SP 800-38A, section 6.1
    ///
    /// **WARNING:** ECB mode does not protect the confidentiality of messages except in vary narrow
    /// circumstances. Use with extreme caution.
    ///
    /// Expected callbacks:
    ///
    /// * `read_key()` to read the AES key.
    /// * `read_input()` to read the input data (plaintext for encryption, ciphertext for
    ///   decryption). Input data must always be a multiple of the [`BLOCK_SIZE`].
    /// * `write_output()` to write the output data (ciphertext for encryption, plaintext for
    ///   decryption). Output data is always the same length as input data.
    ///
    /// Note that an IV is not used in ECB mode.
    Ecb,

    /// Galois/Counter mode, as specified in NIST SP 800-38D
    ///
    /// Expected callbacks:
    ///
    /// * `read_key()` to read the AES key.
    /// * `read_iv()` to read an IV of exactly 12 bytes.
    /// * `read_input()` to read the input data (plaintext for encryption, ciphertext for
    ///   decryption).
    /// * `read_ad()` to read associated data.
    /// * `read_tag()` to read the authentication tag (for decryption).
    /// * `write_output()` to write the output data (ciphertext for encryption, plaintext for
    ///   decryption).
    /// * `write_tag()` to write the authentication tag (for encryption).
    ///
    /// Although AES-GCM technically supports a variety of IV sizes, the NIST spec explicitly
    /// recommends implementations to use a fixed length of 12 bytes. This HIL adopts that
    /// recommendation as a hard requirement.
    Gcm {
        /// Length of authentication tag
        tag_len: TagLength,

        /// Length of associated data, in bytes
        ad_len: usize,
    },
}

/// Encryption or decryption
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Encrypt,
    Decrypt,
}

/// Interface for symmetric encryption and decryption with AES
///
/// This trait defines an abstract interface for performing symmetric cryptography using AES. It
/// supports a variety of cipher modes and exposes a callback-oriented API to maximize flexibility
/// and minimize memory usage.
///
/// Refer to documentation in the [`hil::crypto`][crate::hil::crypto] module for information about
/// the structure and usage of crypto HIL traits in general.
///
/// Depending on the [`Mode`] specified in the call to `crypt()`, the client will receive a
/// different sequence of callbacks, in some cases with different semantics. Refer to documentation
/// of the individual [`Mode`] variants for complete details about the callback sequencing and
/// semantics.
///
/// ## AES-Specific Errors
///
/// The following errors may be returned by this HIL:
///
/// * [`ErrorCode::INVAL`] if the caller provides a counter or nonce of an invalid length
/// * [`ErrorCode::INVAL`] if input is not a multiple of [`BLOCK_SIZE`] (for ciphermodes where this
///   is a requirement)
/// * [`ErrorCode::SIZE`] if the requested operation exceeds the maximum payload size (e.g. due to
///   counter overflow)
/// * [`ErrorCode::FAIL`] if authentication fails (for AEAD modes)
pub trait Aes {
    /// Initiate an AES operation using the given parameters.
    ///
    /// * `len` specifies the length, in bytes, of the plaintext/ciphertext input data
    /// * `key_len` specifies the length of the AES key
    /// * `mode` specifies the block cipher mode to use
    /// * `op` specifies whether to perform encryption or decryption
    ///
    /// Additional, mode-specific parameters are provided as fields in the corresponding [`Mode`]
    /// enum variant.
    ///
    /// All other parameters needed for the requested operation, including the AES key and the input
    /// data itself, will be retrieved asynchronously via [`AesClient`] callbacks. Callback
    /// semantics may differ depending on the selected `mode`; refer to documentation of [`Mode`]
    /// variants for details.
    fn crypt(
        &self,
        len: usize,
        key_len: KeyLength,
        mode: Mode,
        op: Operation,
    ) -> Result<(), ErrorCode>;

    /// Set the client that will receive callbacks for AES operations.
    fn set_client(&self, client: &'static dyn AesClient);
}

pub trait AesClient {
    /// Called by the driver to retrieve the AES key. Client must write the key into the provided
    /// `key` buffer.
    ///
    /// The length of the key is specified by the `key_len` parameter in the initial call to
    /// [`Aes::crypt()`] and determines the size of the `key` buffer provided here. Following
    /// successful return from this callback, the driver shall assume the `key` buffer was
    /// completely filled with an AES key of the appropriate length.
    fn read_key(&self, key: &mut [u8]) -> Result<(), ErrorCode>;

    /// Called by the driver to retrieve the IV for relevant cipher modes. Client must fill the
    /// entire `iv` buffer with the IV value.
    ///
    /// The expected length of the IV varies depending on the cipher mode. Refer to documentation of
    /// [`Mode`] variants for complete details. Following successful return from this callback, the
    /// driver shall assume the `iv` buffer was completely filled.
    fn read_iv(&self, iv: &mut [u8; BLOCK_SIZE]) -> Result<(), ErrorCode>;

    /// Called by the driver to retrieve the initial counter value for an AES-CTR operation. Client
    /// must write the counter value into the provided `ctr` buffer and return the size of the
    /// counter in bytes.
    ///
    /// The counter is interpreted as a little-endian integer.
    ///
    /// The size of the counter must be between 1 and [`BLOCK_SIZE`] bytes, inclusive; if any other
    /// size is returned, the driver shall abort the operation and return [`ErrorCode::INVAL`].
    ///
    /// See [`Mode::Ctr`] for details about counter block construction, incrementing, and wrapping.
    fn read_ctr(&self, ctr: &mut [u8]) -> Result<usize, ErrorCode>;

    /// Called by the driver to retrieve a variable-length nonce for relevant cipher modes. Client
    /// must write the nonce into the provided `nonce` buffer and return its size in bytes.
    ///
    /// The expected nonce length depends on the current cipher mode. Refer to documentation of
    /// [`Mode`] for details.
    fn read_nonce(&self, nonce: &mut [u8]) -> Result<usize, ErrorCode>;

    /// Called by the driver to retrieve associated data for relevant cipher modes. Client must
    /// write the associated data into the provided `ad` buffer, then return the number of bytes
    /// written.
    fn read_ad(&self, ad: &mut [u8]) -> Result<usize, ErrorCode>;

    /// Called by the driver to retrieve input data for the AES operation. Client must write the
    /// input into the `input` buffer, then return the number of bytes written.
    fn read_input(&self, input: &mut [u8]) -> Result<usize, ErrorCode>;

    /// Called by the driver to retrieve the AES-GCM authentication tag for decryption operations.
    fn read_tag(&self, tag: &mut [u8]) -> Result<(), ErrorCode>;

    /// Called by the driver following a successful AES operation to return a "chained" IV value
    /// back to the client. For streaming cipher modes, the value returned via this function can be
    /// used as the input IV for the next operation, allowing easy chaining of multiple operations
    /// without the client needing to manage IV state.
    fn write_iv(&self, iv: &[u8]) -> Result<(), ErrorCode>;

    /// Called by the driver following a successful AES operation to return output data back to the
    /// client.
    fn write_output(&self, output: &[u8]) -> Result<(), ErrorCode>;

    /// Called by the driver following a successful AES-GCM operation to return the authentication
    /// tag back to the client.
    fn write_tag(&self, tag: &[u8]) -> Result<(), ErrorCode>;

    /// Signals the completion of an AES operation initiated by [`Aes::crypt`], with `res`
    /// indicating its outcome.
    fn crypt_done(&self, res: Result<(), ErrorCode>);
}
