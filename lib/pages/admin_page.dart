import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart';
import '../state.dart';
import '../widgets.dart';
import 'async_view.dart';

class AdminPage extends StatelessWidget {
  const AdminPage({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();

    if (!(state.user?.isAdmin ?? false)) {
      return EmptyState(
          icon: Icons.lock_outline, title: l.adminTitle, hint: l.loginTitle);
    }

    return ListView(
      padding: const EdgeInsets.symmetric(vertical: 8),
      children: [
        _Header(l.adminStats),
        AsyncView<AdminStats>(
          what: (l) => l.adminStats,
          loader: () => state.api.adminStats(),
          builder: (context, stats) => Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Wrap(
              children: [
                for (final (label, value) in [
                  (l.adminStatsPosts, '${stats.posts}'),
                  (l.adminStatsUsers, '${stats.users}'),
                  (l.adminStatsTags, '${stats.tags}'),
                  (l.adminStatsComments, '${stats.comments}'),
                  (l.adminStatsPools, '${stats.pools}'),
                  (l.adminStatsFlags, '${stats.flags}'),
                  (l.adminStatsTickets, '${stats.tickets}'),
                  (l.adminStatsDisk, formatBytes(stats.diskBytes)),
                ])
                  SizedBox(
                    width: 160,
                    child: ListTile(
                      dense: true,
                      title: Text(value,
                          style: Theme.of(context).textTheme.titleLarge),
                      subtitle: Text(label),
                    ),
                  ),
              ],
            ),
          ),
        ),
        _Header(l.adminFlags),
        AsyncView<List<PostFlag>>(
          what: (l) => l.adminFlags,
          loader: () => state.api.flags(),
          builder: (context, flags) => flags.isEmpty
              ? ListTile(title: Text(l.emptyGeneric))
              : Column(
                  children: [
                    for (final f in flags)
                      ListTile(
                        dense: true,
                        leading: const Icon(Icons.flag_outlined),
                        title: Text('#${f.postId}'),
                        subtitle: f.reason.isEmpty ? null : Text(f.reason),
                        trailing: TextButton(
                          onPressed: () => state.api.resolveFlag(f.id),
                          child: Text(l.adminResolve),
                        ),
                      ),
                  ],
                ),
        ),
        _Header(l.adminUsers),
        AsyncView<List<User>>(
          what: (l) => l.adminUsers,
          loader: () => state.api.users(),
          builder: (context, users) => Column(
            children: [
              for (final u in users)
                ListTile(
                  dense: true,
                  leading: const Icon(Icons.person_outline),
                  title: Text(u.name),
                  subtitle: Text(u.isAdmin ? 'admin' : 'member'),
                ),
            ],
          ),
        ),
      ],
    );
  }

}

class _Header extends StatelessWidget {
  final String title;
  const _Header(this.title);

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
        child: Text(title,
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
                color: Theme.of(context).colorScheme.primary)),
      );
}
