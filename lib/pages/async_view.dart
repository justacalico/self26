import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../widgets.dart';

/// Runs [loader] once, shows a spinner, then hands the data to [builder].
/// Failures show a retry button.
class AsyncView<T> extends StatefulWidget {
  final Future<T> Function() loader;
  final Widget Function(BuildContext context, T data) builder;
  final String Function(AppLocalizations l)? what;

  const AsyncView({super.key, required this.loader, required this.builder, this.what});

  @override
  State<AsyncView<T>> createState() => _AsyncViewState<T>();
}

class _AsyncViewState<T> extends State<AsyncView<T>> {
  late Future<T> _future = widget.loader();

  void _reload() {
    setState(() {
      _future = widget.loader();
    });
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return FutureBuilder<T>(
      future: _future,
      builder: (context, snap) {
        if (snap.hasError) {
          return ErrorView(
            message: l.loadFailed((widget.what ?? (l) => l.navPosts)(l)),
            retryLabel: l.retry,
            onRetry: _reload,
          );
        }
        final data = snap.data;
        if (data == null) return const LoadingView();
        return widget.builder(context, data);
      },
    );
  }
}
