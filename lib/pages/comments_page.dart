import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart';
import '../state.dart';
import '../widgets.dart';
import 'async_view.dart';

class CommentsPage extends StatelessWidget {
  const CommentsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.read<AppState>();

    return AsyncView<List<Comment>>(
      what: (l) => l.navComments,
      loader: () => state.api.comments(),
      builder: (context, comments) => comments.isEmpty
          ? EmptyState(
              icon: Icons.chat_bubble_outline, title: l.noComments)
          : ListView.builder(
              itemCount: comments.length,
              itemBuilder: (context, i) {
                final c = comments[i];
                return ListTile(
                  dense: true,
                  title: Text(c.body),
                  subtitle: Text('#${c.postId} · ${c.creatorName} · ${c.score}'),
                  onTap: () async {
                    final post = await state.api.post(c.postId);
                    state.openPost(post);
                  },
                );
              },
            ),
    );
  }
}
