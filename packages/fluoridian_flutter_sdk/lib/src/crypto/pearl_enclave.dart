import 'dart:convert';
import 'dart:typed_data';
import 'package:convert/convert.dart';
import 'package:crypto/crypto.dart' as crypto;
import 'package:ed25519_edwards/ed25519_edwards.dart' as ed;

/// Pearl Enclave Cryptographic Keypair and DID Identity Manager.
///
/// Handles decentralized identity format: `did:atlantis:<hex_ed25519_public_key>`.
/// Generates and verifies cryptographic signatures for Proof-of-Health (PoH),
/// token escrow instructions, and asynchronous compute receipts.
class PearlEnclaveSigner {
  final ed.PrivateKey _privateKey;
  final ed.PublicKey publicKey;

  PearlEnclaveSigner._(this._privateKey, this.publicKey);

  /// Generates a new Pearl Enclave signer with a fresh random Ed25519 keypair.
  factory PearlEnclaveSigner.generate() {
    final keyPair = ed.generateKey();
    return PearlEnclaveSigner._(keyPair.privateKey, keyPair.publicKey);
  }

  /// Instantiates a signer from a 32-byte Ed25519 seed.
  factory PearlEnclaveSigner.fromSeed(Uint8List seed) {
    if (seed.length != 32) {
      throw ArgumentError('Seed must be exactly 32 bytes');
    }
    final privateKey = ed.newKeyFromSeed(seed);
    final publicKey = ed.public(privateKey);
    return PearlEnclaveSigner._(privateKey, publicKey);
  }

  /// Instantiates a signer from a 64-byte private key representation.
  factory PearlEnclaveSigner.fromPrivateKeyBytes(Uint8List bytes) {
    if (bytes.length != 64) {
      throw ArgumentError('Private key must be 64 bytes');
    }
    final privateKey = ed.PrivateKey(bytes);
    final publicKey = ed.public(privateKey);
    return PearlEnclaveSigner._(privateKey, publicKey);
  }

  /// Returns the canonical decentralized identity string:
  /// `did:atlantis:<hex_encoded_32_byte_pubkey>`
  String get did {
    final hexPub = hex.encode(publicKey.bytes);
    return 'did:atlantis:$hexPub';
  }

  /// Public key bytes (32 bytes).
  Uint8List get publicKeyBytes => Uint8List.fromList(publicKey.bytes);

  /// Signs an arbitrary message using Ed25519.
  Uint8List sign(Uint8List message) {
    return ed.sign(_privateKey, message);
  }

  /// Signs a UTF-8 text string (such as a PoH challenge or canonical event envelope)
  /// and returns the signature as a lowercase hex string.
  String signString(String message) {
    final messageBytes = utf8.encode(message);
    final signatureBytes = sign(Uint8List.fromList(messageBytes));
    return hex.encode(signatureBytes);
  }

  /// Computes a SHA-256 digest of input data before signing (for large payloads).
  String signDigest(Uint8List data) {
    final digest = crypto.sha256.convert(data);
    final signatureBytes = sign(Uint8List.fromList(digest.bytes));
    return hex.encode(signatureBytes);
  }

  /// Verifies an Ed25519 signature against a public key.
  static bool verify({
    required Uint8List publicKeyBytes,
    required Uint8List message,
    required Uint8List signature,
  }) {
    if (publicKeyBytes.length != 32 || signature.length != 64) {
      return false;
    }
    final pubKey = ed.PublicKey(publicKeyBytes);
    return ed.verify(pubKey, message, signature);
  }

  /// Verifies a string message signature against an Atlantis DID.
  static bool verifyDidSignature({
    required String did,
    required String message,
    required String signatureHex,
  }) {
    if (!did.startsWith('did:atlantis:')) {
      return false;
    }
    try {
      final hexKey = did.substring('did:atlantis:'.length);
      final pubKeyBytes = Uint8List.fromList(hex.decode(hexKey));
      final messageBytes = Uint8List.fromList(utf8.encode(message));
      final sigBytes = Uint8List.fromList(hex.decode(signatureHex));

      return verify(
        publicKeyBytes: pubKeyBytes,
        message: messageBytes,
        signature: sigBytes,
      );
    } catch (_) {
      return false;
    }
  }
}
