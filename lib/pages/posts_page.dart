import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart';
import '../state.dart';
import '../widgets.dart' as w;
import 'async_view.dart';

/// Post grid with tag search and quick Random/Popular/Hot filters.
class PostsPage extends StatelessWidget {
  const PostsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();
    final tags = state.searchTags;

    return Column(
      children: [
        w.SearchBar(
          hint: l.searchHint,
          initial: tags,
          onSubmit: state.setSearch,
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          child: Row(
            children: [
              for (final (label, query) in [
                (l.random, 'order:random'),
                (l.popular, 'order:score'),
                (l.hot, 'order:rank'),
              ])
                Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: ActionChip(
                    label: Text(label),
                    onPressed: () => state.setSearch(query),
                  ),
                ),
            ],
          ),
        ),
        Expanded(
          child: AsyncView<List<Post>>(
            key: ValueKey(tags),
            loader: () => state.api.posts(tags: tags),
            builder: (context, posts) => posts.isEmpty
                ? w.EmptyState(
                    icon: Icons.photo_library_outlined,
                    title: l.noPosts,
                    hint: l.noPostsHint,
                  )
                : w.PostGrid(posts: posts, onOpen: state.openPost),
          ),
        ),
      ],
    );
  }
}
