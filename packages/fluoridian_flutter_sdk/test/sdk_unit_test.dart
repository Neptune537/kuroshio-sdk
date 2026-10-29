import 'package:flutter_test/flutter_test.dart';
import 'package:fluoridian_flutter_sdk/fluoridian_flutter_sdk.dart';

void main() {
  group('Gyre Protocol Models & Serialization', () {
    test('PohMetrics JSON serialization and deserialization', () {
      final metrics = PohMetrics(
        fps: 60.0,
        cpuPct: 18.5,
        vramFreeMb: 8192,
        tempC: 52,
        pingMs: 14.2,
      );

      final jsonMap = metrics.toJson();
      expect(jsonMap['fps'], 60.0);
      expect(jsonMap['cpu_pct'], 18.5);
      expect(jsonMap['vram_free_mb'], 8192);
      expect(jsonMap['temp_c'], 52);
      expect(jsonMap['ping_ms'], 14.2);

      final deserialized = PohMetrics.fromJson(jsonMap);
      expect(deserialized.fps, metrics.fps);
      expect(deserialized.cpuPct, metrics.cpuPct);
      expect(deserialized.vramFreeMb, metrics.vramFreeMb);
      expect(deserialized.tempC, metrics.tempC);
      expect(deserialized.pingMs, metrics.pingMs);
    });

    test('KuroshioEventEnvelope roundtrip JSON encode/decode', () {
      final envelope = KuroshioEventEnvelope(
        id: 'evt_001',
        did: 'did:atlantis:sebastian_edge',
        eventType: 'poh_heartbeat',
        payload: {
          'fps': 60.0,
          'cpu_pct': 18.5,
        },
        signature: 'deadbeef1234',
        timestamp: 1730239000000,
      );

      final jsonMap = envelope.toJson();
      expect(jsonMap['id'], 'evt_001');
      expect(jsonMap['did'], 'did:atlantis:sebastian_edge');
      expect(jsonMap['eventType'], 'poh_heartbeat');
      expect(jsonMap['signature'], 'deadbeef1234');
      expect(jsonMap['timestamp'], 1730239000000);

      final decoded = KuroshioEventEnvelope.fromJson(jsonMap);
      expect(decoded.id, 'evt_001');
      expect(decoded.did, 'did:atlantis:sebastian_edge');
      expect(decoded.eventType, 'poh_heartbeat');
      expect(decoded.payload['fps'], 60.0);
    });

    test('AsynchronousComputeJobRequest serialization matches Triton baker requirements', () {
      final job = AsynchronousComputeJobRequest(
        jobId: 'baker_job_789',
        clientDid: 'did:atlantis:test_client',
        jobType: 'DemElevationTiling',
        payloadUri: 'https://sonar.atlantis.mesh/payloads/heightmap_01.bin',
        inputHash: [1, 2, 3, 4],
        escrowAccountId: 'escrow_acc_456',
        maxLatencySecs: 60,
        rewardCyan: 5.0,
      );

      final jsonMap = job.toJson();
      expect(jsonMap['job_id'], 'baker_job_789');
      expect(jsonMap['client_did'], 'did:atlantis:test_client');
      expect(jsonMap['job_type']['DemElevationTiling']['resolution'], 256);
      expect(jsonMap['payload_uri'], 'https://sonar.atlantis.mesh/payloads/heightmap_01.bin');
      expect(jsonMap['reward_cyan'], 5.0);
    });
  });

  group('Pearl Enclave DID & Signer Cryptography', () {
    test('PearlEnclaveSigner generates valid did:atlantis format', () {
      final signer = PearlEnclaveSigner.generate();
      expect(signer.did.startsWith('did:atlantis:'), isTrue);
      // 'did:atlantis:' (13) + 32-byte hex (64) = 77 chars
      expect(signer.did.length, 77);
    });

    test('PearlEnclaveSigner signs and verifies message successfully', () {
      final signer = PearlEnclaveSigner.generate();
      const message = 'canary_challenge:1730239000000';
      final sigHex = signer.signString(message);

      expect(sigHex.length, 128); // 64 bytes = 128 hex chars

      final verified = PearlEnclaveSigner.verifyDidSignature(
        did: signer.did,
        message: message,
        signatureHex: sigHex,
      );
      expect(verified, isTrue);

      final wrongMessageVerified = PearlEnclaveSigner.verifyDidSignature(
        did: signer.did,
        message: 'tampered_challenge',
        signatureHex: sigHex,
      );
      expect(wrongMessageVerified, isFalse);
    });
  });
}
