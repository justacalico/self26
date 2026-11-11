import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart' as m;
import '../state.dart';
import '../widgets.dart';
import 'async_view.dart';

/// Post detail: image, score/vote/fav actions, tags, metadata, comments.
class PostPage extends StatefulWidget {
  final m.Post post;
  const PostPage({super.key, required this.post});

  @override
  State<PostPage> createState() => _PostPageState();
}

class _PostPageState extends State<PostPage> {
  late m.Post _post = widget.post;
  final _commentController = TextEditingController();
  int _commentRefresh = 0;

  @override
  void dispose() {
    _commentController.dispose();
    super.dispose();
  }

  Future<void> _reloadPost() async {
    final api = context.read<AppState>().api;
    final fresh = await api.post(_post.id);
    if (mounted) setState(() => _post = fresh);
  }

  Future<void> _act(Future<void> Function() action) async {
    try {
      await action();
      await _reloadPost();
    } catch (_) {
      // Unauthenticated actions fail quietly; the vote button state
      // simply does not change.
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.read<AppState>();
    final post = _post;
    final scheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        leading: BackButton(onPressed: state.closeDetail),
        title: Text('#${post.id}'),
        actions: [
          IconButton(
            tooltip: l.addFavourite,
            icon: Icon(post.isFavorited ? Icons.favorite : Icons.favorite_border),
            color: post.isFavorited ? Colors.pinkAccent : null,
            onPressed: () => _act(() => post.isFavorited
                ? state.api.unfavorite(post.id)
                : state.api.favorite(post.id)),
          ),
          IconButton(
            tooltip: l.deletePost,
            icon: const Icon(Icons.delete_outline),
            onPressed: () => _confirmDelete(context, l, state),
          ),
        ],
      ),
      body: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 520),
              child: Center(
                child: post.isVideo
                    ? Container(
                        height: 240,
                        color: scheme.surfaceContainerHighest,
                        child: const Center(
                            child: Icon(Icons.play_circle_outline, size: 64)),
                      )
                    : Image.network(
                        post.sampleUrl.isEmpty ? post.fileUrl : post.sampleUrl,
                        headers: state.imageHeaders,
                        fit: BoxFit.contain,
                        errorBuilder: (_, __, ___) => Container(
                          height: 240,
                          color: scheme.surfaceContainerHighest,
                          child: const Icon(Icons.broken_image_outlined, size: 48),
                        ),
                      ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      IconButton(
                        tooltip: l.voteUp,
                        icon: Icon(Icons.arrow_upward,
                            color: post.myVote > 0
                                ? scheme.primary
                                : scheme.outline),
                        onPressed: () => _act(() =>
                            state.api.vote(post.id, post.myVote > 0 ? 0 : 1)),
                      ),
                      Text('${post.score.total}',
                          style: Theme.of(context).textTheme.titleMedium),
                      IconButton(
                        tooltip: l.voteDown,
                        icon: Icon(Icons.arrow_downward,
                            color: post.myVote < 0
                                ? scheme.error
                                : scheme.outline),
                        onPressed: () => _act(() =>
                            state.api.vote(post.id, post.myVote < 0 ? 0 : -1)),
                      ),
                      const Spacer(),
                      _MetaText(l.postScore, '${post.score.up} / ${post.score.down}'),
                      _MetaText(l.postFavourites, '${post.favCount}'),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    children: [
                      for (final entry in post.tags.entries)
                        for (final tag in entry.value)
                          TagChip(
                            name: tag,
                            category:
                                m.TagInfo.categoryNames.entries
                                    .firstWhere(
                                        (e) => e.value == entry.key,
                                        orElse: () => const MapEntry(0, ''))
                                    .key,
                            onTap: () {
                              state.setSearch(tag);
                              state.select(NavDest.posts);
                            },
                          ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  _MetaText(l.postRating, ratingName(l, post.rating)),
                  _MetaText(l.postUploader, post.uploaderName),
                  if (post.sources.isNotEmpty)
                    _MetaText(l.postSources, post.sources.join(', ')),
                  if (post.parentId != null)
                    _MetaText(l.postParent, '#${post.parentId}'),
                  if (post.children.isNotEmpty)
                    _MetaText(l.postChildren,
                        post.children.map((c) => '#$c').join(', ')),
                  if (post.description.isNotEmpty) ...[
                    const SizedBox(height: 8),
                    Text(l.postDescription,
                        style: Theme.of(context).textTheme.titleSmall),
                    SelectableText(post.description),
                  ],
                  const SizedBox(height: 12),
                  Text('${l.postComments} (${post.commentCount})',
                      style: Theme.of(context).textTheme.titleSmall),
                  const SizedBox(height: 8),
                  _CommentList(postId: post.id, refresh: _commentRefresh),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _commentController,
                          decoration: InputDecoration(
                            hintText: l.commentHint,
                            isDense: true,
                            border: const OutlineInputBorder(),
                          ),
                          onSubmitted: (_) => _postComment(state),
                        ),
                      ),
                      const SizedBox(width: 8),
                      FilledButton.tonal(
                        onPressed: () => _postComment(state),
                        child: Text(l.commentPost),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _postComment(AppState state) async {
    final text = _commentController.text.trim();
    if (text.isEmpty) return;
    try {
      await state.api.addComment(_post.id, text);
      _commentController.clear();
      if (mounted) setState(() => _commentRefresh++);
    } catch (_) {
      // Comment failed — leave the draft in place.
    }
  }

  void _confirmDelete(BuildContext context, AppLocalizations l, AppState state) {
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.deletePost),
        content: Text(l.deleteConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              state.api.deletePost(_post.id).then((_) {}).catchError((_) {});
              state.closeDetail();
            },
            child: Text(l.confirm),
          ),
        ],
      ),
    );
  }
}

String ratingName(AppLocalizations l, String rating) => switch (rating) {
      'e' => l.ratingExplicit,
      'q' => l.ratingQuestionable,
      _ => l.ratingSafe,
    };

class _MetaText extends StatelessWidget {
  final String label;
  final String value;
  const _MetaText(this.label, this.value);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(right: 16, top: 4),
      child: Text.rich(
        TextSpan(children: [
          TextSpan(
              text: '$label: ',
              style: TextStyle(color: Theme.of(context).colorScheme.outline)),
          TextSpan(text: value),
        ]),
      ),
    );
  }
}

class _CommentList extends StatelessWidget {
  final int postId;
  final int refresh;
  const _CommentList({required this.postId, required this.refresh});

  @override
  Widget build(BuildContext context) {
    final state = context.read<AppState>();
    final l = AppLocalizations.of(context);
    return AsyncView<List<m.Comment>>(
      key: ValueKey('comments-$postId-$refresh'),
      loader: () => state.api.comments(postId: postId),
      builder: (context, comments) {
        if (comments.isEmpty) {
          return Text(l.noComments,
              style: TextStyle(color: Theme.of(context).colorScheme.outline));
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final c in comments)
              ListTile(
                dense: true,
                contentPadding: EdgeInsets.zero,
                title: Text(c.body),
                subtitle: Text('${c.creatorName} · ${c.score}'),
                trailing: state.user?.isAdmin == true ||
                        state.user?.id == c.creatorId
                    ? IconButton(
                        icon: const Icon(Icons.delete_outline, size: 18),
                        tooltip: l.commentDelete,
                        onPressed: () => state.api.deleteComment(c.id),
                      )
                    : null,
              ),
          ],
        );
      },
    );
  }
}
