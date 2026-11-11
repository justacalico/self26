import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart';
import '../state.dart';
import '../widgets.dart';
import 'async_view.dart';

class PoolsPage extends StatelessWidget {
  const PoolsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();
    final selected = state.selectedPool;
    if (selected != null) return _PoolDetail(pool: selected);

    return Column(
      children: [
        if (state.user != null)
          Align(
            alignment: Alignment.centerRight,
            child: Padding(
              padding: const EdgeInsets.all(8),
              child: FilledButton.tonalIcon(
                icon: const Icon(Icons.add),
                label: Text(l.poolsNew),
                onPressed: () => _createPool(context, l, state),
              ),
            ),
          ),
        Expanded(
          child: AsyncView<List<Pool>>(
            what: (l) => l.navPools,
            loader: () => state.api.pools(),
            builder: (context, pools) => pools.isEmpty
                ? EmptyState(
                    icon: Icons.collections_bookmark_outlined,
                    title: l.noPools,
                  )
                : ListView.builder(
                    itemCount: pools.length,
                    itemBuilder: (context, i) {
                      final pool = pools[i];
                      return ListTile(
                        leading: const Icon(Icons.collections_bookmark_outlined),
                        title: Text(pool.name.replaceAll('_', ' ')),
                        subtitle: pool.description.isEmpty
                            ? null
                            : Text(pool.description,
                                maxLines: 1, overflow: TextOverflow.ellipsis),
                        trailing: Text(l.poolPosts(pool.postIds.length)),
                        onTap: () => state.openPool(pool),
                      );
                    },
                  ),
          ),
        ),
      ],
    );
  }

  void _createPool(BuildContext context, AppLocalizations l, AppState state) {
    final name = TextEditingController();
    final desc = TextEditingController();
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.poolsNew),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: name,
              decoration: InputDecoration(labelText: l.poolName),
            ),
            TextField(
              controller: desc,
              decoration: InputDecoration(labelText: l.poolDescription),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              state.api
                  .createPool(name.text.trim(), desc.text.trim())
                  .then((pool) => state.openPool(pool))
                  .catchError((_) {});
            },
            child: Text(l.poolCreate),
          ),
        ],
      ),
    );
  }
}

class _PoolDetail extends StatelessWidget {
  final Pool pool;
  const _PoolDetail({required this.pool});

  @override
  Widget build(BuildContext context) {
    final state = context.read<AppState>();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ListTile(
          leading: BackButton(onPressed: state.closeDetail),
          title: Text(pool.name.replaceAll('_', ' ')),
          subtitle: pool.description.isEmpty ? null : Text(pool.description),
        ),
        Expanded(
          child: AsyncView<List<Post>>(
            loader: () => state.api.poolPosts(pool.id),
            builder: (context, posts) => posts.isEmpty
                ? EmptyState(
                    icon: Icons.photo_library_outlined,
                    title: AppLocalizations.of(context).noPosts,
                  )
                : PostGrid(posts: posts, onOpen: state.openPost),
          ),
        ),
      ],
    );
  }
}
