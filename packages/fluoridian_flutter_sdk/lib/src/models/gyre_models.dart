/// Canonical wire protocol models for the Kuroshio Gyre & Atlantis.
library fluoridian_models;

enum GyreNodeTier {
  abyssalAnchor,
  pelagicWorker,
  epipelagicEdge,
}

class PohMetrics {
  final double fps;
  final double cpuPct;
  final int vramFreeMb;
  final int tempC;
  final double? pingMs;

  const PohMetrics({
    required this.fps,
    required this.cpuPct,
    required this.vramFreeMb,
    required this.tempC,
    this.pingMs,
  });

  Map<String, dynamic> toJson() => {
    'fps': fps,
    'cpu_pct': cpuPct,
    'vram_free_mb': vramFreeMb,
    'temp_c': tempC,
    if (pingMs != null) 'ping_ms': pingMs,
  };

  factory PohMetrics.fromJson(Map<String, dynamic> json) => PohMetrics(
    fps: (json['fps'] as num).toDouble(),
    cpuPct: (json['cpu_pct'] as num).toDouble(),
    vramFreeMb: json['vram_free_mb'] as int,
    tempC: json['temp_c'] as int,
    pingMs: json['ping_ms'] != null ? (json['ping_ms'] as num).toDouble() : null,
  );
}

class KuroshioEventEnvelope {
  final String id;
  final String did;
  final String eventType;
  final Map<String, dynamic> payload;
  final String signature;
  final int timestamp;

  const KuroshioEventEnvelope({
    required this.id,
    required this.did,
    required this.eventType,
    required this.payload,
    required this.signature,
    required this.timestamp,
  });

  Map<String, dynamic> toJson() => {
    'id': id,
    'did': did,
    'eventType': eventType,
    'payload': payload,
    'signature': signature,
    'timestamp': timestamp,
  };

  factory KuroshioEventEnvelope.fromJson(Map<String, dynamic> json) => KuroshioEventEnvelope(
    id: json['id'] as String,
    did: json['did'] as String,
    eventType: json['eventType'] as String,
    payload: Map<String, dynamic>.from(json['payload'] as Map),
    signature: json['signature'] as String,
    timestamp: json['timestamp'] as int,
  );
}

class AsynchronousComputeJobRequest {
  final String jobId;
  final String clientDid;
  final String jobType;
  final String payloadUri;
  final List<int> inputHash;
  final String escrowAccountId;
  final int maxLatencySecs;
  final double rewardCyan;

  const AsynchronousComputeJobRequest({
    required this.jobId,
    required this.clientDid,
    required this.jobType,
    required this.payloadUri,
    required this.inputHash,
    required this.escrowAccountId,
    required this.maxLatencySecs,
    required this.rewardCyan,
  });

  Map<String, dynamic> toJson() => {
    'job_id': jobId,
    'client_did': clientDid,
    'job_type': {
      if (jobType == 'DemElevationTiling') 'DemElevationTiling': {'resolution': 256, 'target_lod': 2}
      else 'GaussianSplatHullSynthesis': {'max_convex_hulls': 8}
    },
    'payload_uri': payloadUri,
    'input_hash': inputHash,
    'escrow_account_id': escrowAccountId,
    'max_latency_secs': maxLatencySecs,
    'reward_cyan': rewardCyan,
  };
}
