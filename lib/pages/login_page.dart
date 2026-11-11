import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state.dart';

/// Sign-in form: password login, or an API key for headless clients.
class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _name = TextEditingController();
  final _password = TextEditingController();
  final _keyName = TextEditingController();
  final _key = TextEditingController();
  bool _busy = false;
  bool _failed = false;

  @override
  void dispose() {
    _name.dispose();
    _password.dispose();
    _keyName.dispose();
    _key.dispose();
    super.dispose();
  }

  Future<void> _run(Future<void> Function() action) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _failed = false;
    });
    try {
      await action();
      if (mounted) Navigator.of(context).maybePop();
    } catch (_) {
      if (mounted) setState(() => _failed = true);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.read<AppState>();

    return Scaffold(
      appBar: AppBar(title: Text(l.loginTitle)),
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 360),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                TextField(
                  controller: _name,
                  decoration: InputDecoration(
                    labelText: l.loginName,
                    border: const OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _password,
                  obscureText: true,
                  decoration: InputDecoration(
                    labelText: l.loginPassword,
                    border: const OutlineInputBorder(),
                  ),
                  onSubmitted: (_) =>
                      _run(() => state.signIn(_name.text.trim(), _password.text)),
                ),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: _busy
                      ? null
                      : () => _run(() =>
                          state.signIn(_name.text.trim(), _password.text)),
                  child: _busy
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2))
                      : Text(l.loginSubmit),
                ),
                const Divider(height: 40),
                TextField(
                  controller: _keyName,
                  decoration: InputDecoration(
                    labelText: l.loginName,
                    border: const OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _key,
                  decoration: InputDecoration(
                    labelText: l.loginApiKey,
                    hintText: l.loginApiKeyHint,
                    border: const OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 16),
                OutlinedButton(
                  onPressed: _busy
                      ? null
                      : () => _run(() => state.signInWithKey(
                          _keyName.text.trim(), _key.text.trim())),
                  child: Text(l.loginUseKey),
                ),
                if (_failed)
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: Text(
                      l.loginFailed,
                      style:
                          TextStyle(color: Theme.of(context).colorScheme.error),
                      textAlign: TextAlign.center,
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
