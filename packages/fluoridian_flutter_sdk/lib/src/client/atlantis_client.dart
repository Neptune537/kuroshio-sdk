import 'dart:convert';
import 'package:crypto/crypto.dart';
import 'package:uuid/uuid.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import '../models/gyre_models.dart';

/// High-level client connecting directly to Atlantis Anchor services.
class AtlantisClient {
  final String anchorWsUrl;
  final String homeNodeHttpUrl;
  final String clientDid;
  WebSocketChannel? _channel;
  bool _isConnected = false;

  bool get isConnected => _isConnected;

  AtlantisClient({
    this.anchorWsUrl = 'ws://localhost:8080/ws/sonar',
    this.homeNodeHttpUrl = 'http://localhost:9001/api',
    this.clientDid = 'did:atlantis:fluoridian-flutter-client',
  });

  /// Connects to the Triton Sonar WebSocket relay.
  Future<void> connect() async {
    try {
      _channel = WebSocketChannel.connect(Uri.parse(anchorWsUrl));
      _isConnected = true;
      // Subscribe to global telemetry and receipts
      _sendJson({
        'action': 'subscribe',
        'topic': 'gyre:telemetry:global',
      });
      _sendJson({
        'action': 'subscribe',
        'topic': 'compute:receipts',
      });
    } catch (e) {
      _isConnected = false;
      rethrow;
    }
  }

  /// Dispatches an AsynchronousComputeJob over the Sonar WebSocket relay.
  Future<void> submitAsyncComputeJob(AsynchronousComputeJobRequest job) async {
    if (!_isConnected || _channel == null) {
      throw StateError('Cannot dispatch compute job: not connected to Triton Sonar Relay');
    }

    final envelope = KuroshioEventEnvelope(
      id: 'job_${job.jobId}',
      did: clientDid,
      eventType: 'ASYNC_COMPUTE_SUBMIT',
      payload: {
        'node_tier': 'Epipelagic',
        'task_id': job.jobId,
        'task_type': job.jobType,
        'async_compute_job': job.toJson(),
      },
      signature: 'ed25519_client_signature',
      timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
    );

    _sendJson({
      'action': 'publish',
      'topic': 'compute:jobs',
      'event': envelope.toJson(),
    });
  }

  void _sendJson(Map<String, dynamic> data) {
    if (_channel != null) {
      _channel!.sink.add(jsonEncode(data));
    }
  }

  /// Disconnects from the Sonar Relay.
  Future<void> disconnect() async {
    await _channel?.sink.close();
    _isConnected = false;
  }
}
