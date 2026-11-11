import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state.dart';
import '../widgets.dart';

/// The live implementation; tests replace this with a fake.
Future<FilePickerResult?> Function() pickUploadFile = () =>
    FilePicker.platform.pickFiles(type: FileType.any, withData: true);

class UploadPage extends StatefulWidget {
  const UploadPage({super.key});

  @override
  State<UploadPage> createState() => _UploadPageState();
}

class _UploadPageState extends State<UploadPage> {
  final _tags = TextEditingController();
  final _source = TextEditingController();
  final _description = TextEditingController();
  PlatformFile? _file;
  bool _uploading = false;
  String _rating = 's';
  String? _message;

  @override
  void dispose() {
    _tags.dispose();
    _source.dispose();
    _description.dispose();
    super.dispose();
  }

  Future<void> _pick() async {
    final result = await pickUploadFile();
    if (result != null && result.files.isNotEmpty) {
      setState(() => _file = result.files.first);
    }
  }

  Future<void> _submit(AppState state, AppLocalizations l) async {
    final file = _file;
    if (file == null || file.bytes == null || _uploading) return;
    setState(() {
      _uploading = true;
      _message = null;
    });
    try {
      final post = await state.api.upload(
        file.bytes!,
        file.name,
        tags: _tags.text,
        rating: _rating,
        source: _source.text,
        description: _description.text,
      );
      if (!mounted) return;
      setState(() => _message = l.uploadSuccess(post.id));
      state.openPost(post);
    } catch (_) {
      if (mounted) setState(() => _message = l.uploadFailed);
    } finally {
      if (mounted) setState(() => _uploading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();

    if (state.user == null) {
      return EmptyState(icon: Icons.login, title: l.uploadSignIn);
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            OutlinedButton.icon(
              icon: const Icon(Icons.attach_file),
              label: Text(_file?.name ?? l.uploadChoose),
              onPressed: _pick,
            ),
            if (_file != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  '${_file!.name} · ${((_file!.bytes?.length ?? 0) / 1024).toStringAsFixed(0)} KB',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
            const SizedBox(height: 12),
            TextField(
              controller: _tags,
              decoration: InputDecoration(
                labelText: l.uploadTags,
                hintText: l.uploadTagsHint,
                border: const OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _source,
              decoration: InputDecoration(
                labelText: l.uploadSource,
                border: const OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _description,
              maxLines: 3,
              decoration: InputDecoration(
                labelText: l.uploadDescription,
                border: const OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            SegmentedButton<String>(
              segments: [
                ButtonSegment(value: 's', label: Text(l.ratingSafe)),
                ButtonSegment(value: 'q', label: Text(l.ratingQuestionable)),
                ButtonSegment(value: 'e', label: Text(l.ratingExplicit)),
              ],
              selected: {_rating},
              onSelectionChanged: (s) => setState(() => _rating = s.first),
            ),
            const SizedBox(height: 16),
            FilledButton.icon(
              icon: _uploading
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2))
                  : const Icon(Icons.upload),
              label: Text(l.uploadSubmit),
              onPressed: _file == null || _uploading
                  ? null
                  : () => _submit(state, l),
            ),
            if (_message != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(_message!),
              ),
          ],
        ),
      ),
    );
  }
}
