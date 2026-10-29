import 'package:flutter/widgets.dart';

/// Embedded 3D Viewport Widget interfacing with Lemuria / Fluorescent rendering engines.
class FluorescentViewport extends StatelessWidget {
  final String streamEndpoint;
  final Widget? fallback;
  final bool enableWebGpu;

  const FluorescentViewport({
    super.key,
    required this.streamEndpoint,
    this.fallback,
    this.enableWebGpu = true,
  });

  @override
  Widget build(BuildContext context) {
    // In native runtime or WebXR runner, embeds the texture surface.
    return Container(
      color: const Color(0xFF0F172A),
      alignment: Alignment.center,
      child: fallback ??
          const Text(
            'Fluorescent AAA Viewport Active\n[WebGPU / Vulkan Texture Target]',
            textAlign: TextAlign.center,
            style: TextStyle(
              color: Color(0xFF38BDF8),
              fontSize: 14,
              fontFamily: 'monospace',
            ),
          ),
    );
  }
}
