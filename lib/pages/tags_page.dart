import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../models.dart';
import '../state.dart';
import '../widgets.dart' as w;
import 'async_view.dart';

class TagsPage extends StatefulWidget {
  const TagsPage({super.key});

  @override
  State<TagsPage> createState() => _TagsPageState();
}

class _TagsPageState extends State<TagsPage> {
  String _query = '';

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.read<AppState>();

    return Column(
      children: [
        w.SearchBar(
          hint: l.navTags,
          initial: _query,
          onSubmit: (q) => setState(() => _query = q),
        ),
        Expanded(
          child: AsyncView<List<TagInfo>>(
            key: ValueKey(_query),
            what: (l) => l.navTags,
            loader: () => state.api.tags(query: _query),
            builder: (context, tags) => tags.isEmpty
                ? w.EmptyState(icon: Icons.sell_outlined, title: l.emptyGeneric)
                : ListView.builder(
                    itemCount: tags.length,
                    itemBuilder: (context, i) {
                      final tag = tags[i];
                      return ListTile(
                        dense: true,
                        leading: Icon(Icons.sell_outlined,
                            size: 16, color: w.tagCategoryColor(tag.category)),
                        title: Text(
                          tag.name.replaceAll('_', ' '),
                          style:
                              TextStyle(color: w.tagCategoryColor(tag.category)),
                        ),
                        subtitle: Text(tag.categoryName),
                        trailing: Text('${tag.postCount}'),
                        onTap: () {
                          state.setSearch(tag.name);
                          state.select(NavDest.posts);
                        },
                      );
                    },
                  ),
          ),
        ),
      ],
    );
  }
}
