import 'package:flutter/material.dart';

class ShowcaseCard extends StatelessWidget {
  const ShowcaseCard(
      {super.key, required this.painter, required this.caption});

  final CustomPainter painter;
  final String caption;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: 300,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            height: 200,
            width: 300,
            clipBehavior: Clip.antiAlias,
            decoration: BoxDecoration(
              color: scheme.surfaceContainerLow,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: scheme.outlineVariant),
            ),
            child: CustomPaint(painter: painter),
          ),
          const SizedBox(height: 8),
          Text(caption, style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
    );
  }
}

const _tileColors = [
  Color(0xFF3A6EA5),
  Color(0xFF6E5F9E),
  Color(0xFF4C8055),
  Color(0xFF8A5A44),
  Color(0xFF2F7E8C),
  Color(0xFF946B2D),
  Color(0xFF7A4A6B),
  Color(0xFF50683B),
  Color(0xFF415F8F),
];

const _ratingColors = [
  Color(0xFF3E9E4F),
  Color(0xFFD9A33A),
  Color(0xFFC94F4F),
];

RRect _pill(Rect r) =>
    RRect.fromRectAndRadius(r, const Radius.circular(6));

class PostGridPainter extends CustomPainter {
  PostGridPainter({required this.scheme});

  final ColorScheme scheme;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint();
    const pad = 14.0;

    paint.color = scheme.surfaceContainerHigh;
    canvas.drawRRect(
        _pill(Rect.fromLTWH(pad, pad, size.width - pad * 2, 24)), paint);
    paint.color = scheme.onSurfaceVariant.withValues(alpha: 0.8);
    canvas.drawCircle(Offset(pad + 13, pad + 12), 4.5, paint);
    paint.color = scheme.onSurfaceVariant.withValues(alpha: 0.45);
    canvas.drawRRect(
        _pill(Rect.fromLTWH(pad + 24, pad + 8, 110, 8)), paint);

    final gridTop = pad + 34;
    final gridBottom = size.height - 34;
    const cols = 3;
    const gap = 8.0;
    final tileW = (size.width - pad * 2 - gap * (cols - 1)) / cols;
    final tileH = (gridBottom - gridTop - gap) / 2;

    var i = 0;
    for (var row = 0; row < 2; row++) {
      for (var col = 0; col < cols; col++) {
        final x = pad + col * (tileW + gap);
        final y = gridTop + row * (tileH + gap);
        final rect = Rect.fromLTWH(x, y, tileW, tileH);
        paint.color = _tileColors[i % _tileColors.length];
        canvas.drawRRect(
            RRect.fromRectAndRadius(rect, const Radius.circular(10)), paint);

        paint.color = _ratingColors[i % _ratingColors.length];
        canvas.drawRRect(
            RRect.fromRectAndRadius(
                Rect.fromLTWH(x + 6, y + 6, 14, 5), const Radius.circular(2)),
            paint);

        if (i % 3 == 1) {
          paint.color = Colors.white.withValues(alpha: 0.85);
          canvas.drawCircle(Offset(x + tileW - 12, y + 11), 4, paint);
        }
        i++;
      }
    }

    var chipX = pad;
    final chipY = size.height - 24.0;
    for (final w in [44.0, 62.0, 52.0, 36.0]) {
      paint.color = scheme.primary.withValues(alpha: 0.22);
      canvas.drawRRect(
          _pill(Rect.fromLTWH(chipX, chipY, w, 14)), paint);
      chipX += w + 7;
    }
  }

  @override
  bool shouldRepaint(PostGridPainter old) => old.scheme != scheme;
}

class ApiCardPainter extends CustomPainter {
  ApiCardPainter({required this.scheme});

  final ColorScheme scheme;

  static const _key = Color(0xFF79C0FF);
  static const _str = Color(0xFF7EE787);
  static const _num = Color(0xFFFFA657);
  static const _dim = Color(0xFF8B949E);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint();
    const pad = 14.0;

    paint.color = const Color(0xFF161B22);
    canvas.drawRRect(
        RRect.fromRectAndRadius(
            Rect.fromLTWH(pad, pad, size.width - pad * 2, size.height - pad * 2),
            const Radius.circular(10)),
        paint);

    paint.color = scheme.primary;
    canvas.drawRRect(
        _pill(Rect.fromLTWH(pad + 12, pad + 11, 34, 14)), paint);
    paint.color = _dim;
    canvas.drawRRect(
        _pill(Rect.fromLTWH(pad + 52, pad + 14, 110, 8)), paint);
    paint.color = const Color(0xFF3FB950);
    canvas.drawCircle(Offset(size.width - pad - 16, pad + 17), 4, paint);

    final lines = <List<(Color, double)>>[
      [(_dim, 10)],
      [(_key, 26), (_num, 44)],
      [(_key, 40), (_str, 96)],
      [(_key, 44), (_str, 18)],
      [(_key, 34), (_num, 56)],
      [(_key, 50), (_dim, 26), (_str, 40)],
      [(_dim, 10)],
    ];

    var y = pad + 40.0;
    const rowH = 19.0;
    for (final line in lines) {
      var x = pad + 14.0;
      for (final (color, w) in line) {
        paint.color = color;
        canvas.drawRRect(_pill(Rect.fromLTWH(x, y + 4, w, 8)), paint);
        x += w + 7;
      }
      y += rowH;
    }
  }

  @override
  bool shouldRepaint(ApiCardPainter old) => old.scheme != scheme;
}

class ShellPainter extends CustomPainter {
  ShellPainter({required this.scheme});

  final ColorScheme scheme;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint();
    const pad = 14.0;
    const sidebarW = 74.0;

    paint.color = scheme.surfaceContainerHigh;
    canvas.drawRRect(
        RRect.fromRectAndRadius(
            Rect.fromLTWH(pad, pad, sidebarW, size.height - pad * 2),
            const Radius.circular(10)),
        paint);

    paint.color = scheme.primary;
    canvas.drawCircle(Offset(pad + sidebarW / 2, pad + 16), 8, paint);

    var itemY = pad + 36.0;
    for (var i = 0; i < 4; i++) {
      paint.color = i == 0
          ? scheme.primary.withValues(alpha: 0.85)
          : scheme.onSurfaceVariant.withValues(alpha: 0.35);
      canvas.drawRRect(
          _pill(Rect.fromLTWH(pad + 10, itemY, sidebarW - 20, 12)), paint);
      itemY += 22;
    }

    final contentX = pad + sidebarW + 10;
    final contentW = size.width - contentX - pad;

    paint.color = scheme.surfaceContainerHigh;
    canvas.drawRRect(
        _pill(Rect.fromLTWH(contentX, pad, contentW, 24)), paint);
    paint.color = scheme.onSurfaceVariant.withValues(alpha: 0.45);
    canvas.drawRRect(
        _pill(Rect.fromLTWH(contentX + 10, pad + 8, 80, 8)), paint);
    paint.color = scheme.primary;
    canvas.drawRRect(
        _pill(Rect.fromLTWH(
            contentX + contentW - 44, pad + 6, 36, 12)), paint);

    const cols = 2;
    const gap = 8.0;
    final tileW = (contentW - gap) / cols;
    final tileH = (size.height - pad * 2 - 32 - gap) / 2;
    var i = 0;
    for (var row = 0; row < 2; row++) {
      for (var col = 0; col < cols; col++) {
        final x = contentX + col * (tileW + gap);
        final y = pad + 32 + row * (tileH + gap);
        paint.color = _tileColors[(i + 3) % _tileColors.length]
            .withValues(alpha: 0.85);
        canvas.drawRRect(
            RRect.fromRectAndRadius(
                Rect.fromLTWH(x, y, tileW, tileH), const Radius.circular(10)),
            paint);
        i++;
      }
    }
  }

  @override
  bool shouldRepaint(ShellPainter old) => old.scheme != scheme;
}
