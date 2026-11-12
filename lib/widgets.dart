import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'models.dart';
import 'state.dart';

/// Colour used for a post's rating letter, e621-style.
Color ratingColor(String rating) => switch (rating) {
      'e' => const Color(0xFFE45F5F),
      'q' => const Color(0xFFFFA823),
      _ => const Color(0xFF3E9E3E),
    };

/// e621 tag category colours.
Color tagCategoryColor(int category) => switch (category) {
      1 => const Color(0xFFF2AC08), // artist
      3 => const Color(0xFFDD00DD), // copyright
      4 => const Color(0xFF00AA00), // character
      5 => const Color(0xFFED5D1F), // species
      7 => const Color(0xFFFFA0A0), // meta
      8 => const Color(0xFF35D6ED), // lore
      _ => const Color(0xFF0073FF), // general
    };

class EmptyState extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? hint;

  const EmptyState({super.key, required this.icon, required this.title, this.hint});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 56, color: scheme.outline),
          const SizedBox(height: 12),
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          if (hint != null) ...[
            const SizedBox(height: 4),
            Text(hint!, style: TextStyle(color: scheme.outline)),
          ],
        ],
      ),
    );
  }
}

class LoadingView extends StatelessWidget {
  const LoadingView({super.key});

  @override
  Widget build(BuildContext context) =>
      const Center(child: CircularProgressIndicator());
}

class ErrorView extends StatelessWidget {
  final String message;
  final String retryLabel;
  final VoidCallback onRetry;

  const ErrorView({
    super.key,
    required this.message,
    required this.retryLabel,
    required this.onRetry,
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(message),
          const SizedBox(height: 12),
          FilledButton.tonal(onPressed: onRetry, child: Text(retryLabel)),
        ],
      ),
    );
  }
}

/// One post tile: preview image, rating stripe and score badge.
class PostTile extends StatelessWidget {
  final Post post;
  final VoidCallback? onTap;

  const PostTile({super.key, required this.post, this.onTap});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Card(
      clipBehavior: Clip.antiAlias,
      margin: EdgeInsets.zero,
      child: InkWell(
        onTap: onTap,
        child: Stack(
          fit: StackFit.expand,
          children: [
            if (post.isVideo)
              Container(
                color: scheme.surfaceContainerHighest,
                child: const Center(child: Icon(Icons.play_circle_outline, size: 40)),
              )
            else
              Image.network(
                post.previewUrl.isEmpty ? post.fileUrl : post.previewUrl,
                fit: BoxFit.cover,
                headers: context.read<AppState>().imageHeaders,
                errorBuilder: (_, __, ___) => Container(
                  color: scheme.surfaceContainerHighest,
                  child: const Icon(Icons.broken_image_outlined),
                ),
                loadingBuilder: imagePlaceholder,
              ),
            Positioned(
              left: 0,
              right: 0,
              bottom: 0,
              child: Container(
                color: Colors.black.withAlpha(120),
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                child: Row(
                  children: [
                    Text(
                      post.rating.toUpperCase(),
                      style: TextStyle(
                        color: ratingColor(post.rating),
                        fontWeight: FontWeight.bold,
                        fontSize: 11,
                      ),
                    ),
                    const SizedBox(width: 6),
                    const Icon(Icons.arrow_upward,
                        size: 12, color: Colors.white70),
                    Text(' ${post.score.total}',
                        style: const TextStyle(
                            color: Colors.white70, fontSize: 11)),
                    const Spacer(),
                    const Icon(Icons.favorite,
                        size: 12, color: Colors.white70),
                    Text(' ${post.favCount}',
                        style: const TextStyle(
                            color: Colors.white70, fontSize: 11)),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Responsive grid of post tiles.
class PostGrid extends StatelessWidget {
  final List<Post> posts;
  final void Function(Post) onOpen;

  const PostGrid({super.key, required this.posts, required this.onOpen});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final cols = (constraints.maxWidth / 180).floor().clamp(2, 8);
        return GridView.builder(
          padding: const EdgeInsets.all(8),
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: cols,
            mainAxisSpacing: 8,
            crossAxisSpacing: 8,
            childAspectRatio: 1,
          ),
          itemCount: posts.length,
          itemBuilder: (context, i) =>
              PostTile(post: posts[i], onTap: () => onOpen(posts[i])),
        );
      },
    );
  }
}

/// A single tag chip in e621 category colours.
class TagChip extends StatelessWidget {
  final String name;
  final int category;
  final VoidCallback? onTap;

  const TagChip({super.key, required this.name, this.category = 0, this.onTap});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(right: 4, bottom: 4),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(6),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          child: Text(
            name.replaceAll('_', ' '),
            style: TextStyle(color: tagCategoryColor(category), fontSize: 13),
          ),
        ),
      ),
    );
  }
}

/// Search field + Random/Popular/Hot actions shown above post lists.
class SearchBar extends StatefulWidget {
  final String hint;
  final String initial;
  final void Function(String) onSubmit;

  const SearchBar({
    super.key,
    required this.hint,
    required this.initial,
    required this.onSubmit,
  });

  @override
  State<SearchBar> createState() => _SearchBarState();
}

class _SearchBarState extends State<SearchBar> {
  late final TextEditingController _controller =
      TextEditingController(text: widget.initial);

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: TextField(
        controller: _controller,
        decoration: InputDecoration(
          hintText: widget.hint,
          prefixIcon: const Icon(Icons.search),
          isDense: true,
          border: const OutlineInputBorder(),
          suffixIcon: IconButton(
            icon: const Icon(Icons.clear),
            onPressed: () {
              _controller.clear();
              widget.onSubmit('');
            },
          ),
        ),
        textInputAction: TextInputAction.search,
        onSubmitted: widget.onSubmit,
      ),
    );
  }
}

Widget imagePlaceholder(
        BuildContext context, Widget child, ImageChunkEvent? progress) =>
    progress == null
        ? child
        : Container(
            color: Theme.of(context).colorScheme.surfaceContainerHighest);

String formatBytes(int bytes) {
  if (bytes > 1 << 30) return '${(bytes / (1 << 30)).toStringAsFixed(1)} GB';
  if (bytes > 1 << 20) return '${(bytes / (1 << 20)).toStringAsFixed(1)} MB';
  if (bytes > 1 << 10) return '${(bytes / (1 << 10)).toStringAsFixed(1)} KB';
  return '$bytes B';
}
