import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart';
import '../state.dart';
import '../widgets.dart';
import 'async_view.dart';

class FavoritesPage extends StatelessWidget {
  const FavoritesPage({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();
    return AsyncView<List<Post>>(
      what: (l) => l.navFavourites,
      loader: () => state.api.favorites(),
      builder: (context, posts) => posts.isEmpty
          ? EmptyState(icon: Icons.favorite_border, title: l.noFavourites)
          : PostGrid(posts: posts, onOpen: state.openPost),
    );
  }
}
