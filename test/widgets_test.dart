import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:self26/models.dart';
import 'package:self26/widgets.dart' as w;

import 'helpers.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('ratingColor picks per rating', () {
    expect(w.ratingColor('e'), const Color(0xFFE45F5F));
    expect(w.ratingColor('q'), const Color(0xFFFFA823));
    expect(w.ratingColor('s'), const Color(0xFF3E9E3E));
  });

  test('tagCategoryColor picks per category', () {
    expect(w.tagCategoryColor(1), const Color(0xFFF2AC08));
    expect(w.tagCategoryColor(3), const Color(0xFFDD00DD));
    expect(w.tagCategoryColor(4), const Color(0xFF00AA00));
    expect(w.tagCategoryColor(5), const Color(0xFFED5D1F));
    expect(w.tagCategoryColor(7), const Color(0xFFFFA0A0));
    expect(w.tagCategoryColor(8), const Color(0xFF35D6ED));
    expect(w.tagCategoryColor(0), const Color(0xFF0073FF));
    expect(w.tagCategoryColor(2), const Color(0xFF0073FF));
  });

  testWidgets('EmptyState shows hint when provided', (tester) async {
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(
        body: w.EmptyState(
            icon: Icons.abc, title: 'empty', hint: 'do something'),
      ),
    ));
    expect(find.text('empty'), findsOneWidget);
    expect(find.text('do something'), findsOneWidget);

    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: w.EmptyState(icon: Icons.abc, title: 'empty')),
    ));
    expect(find.text('do something'), findsNothing);
  });

  testWidgets('LoadingView and ErrorView', (tester) async {
    var retried = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: w.ErrorView(
          message: 'bad',
          retryLabel: 'retry',
          onRetry: () => retried = true,
        ),
      ),
    ));
    expect(find.text('bad'), findsOneWidget);
    await tester.tap(find.text('retry'));
    expect(retried, isTrue);

    await tester.pumpWidget(
        const MaterialApp(home: Scaffold(body: w.LoadingView())));
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('PostTile renders video and image variants', (tester) async {
    final state = await makeState();
    final video = Post.fromV2(postJson(1, video: true));
    final image = Post.fromV2(postJson(2));

    await tester.pumpWidget(ChangeNotifierProvider.value(
      value: state,
      child: MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              SizedBox(
                  width: 200, height: 200, child: w.PostTile(post: video)),
              SizedBox(
                  width: 200, height: 200, child: w.PostTile(post: image)),
            ],
          ),
        ),
      ),
    ));
    await tester.pump();
    expect(find.byIcon(Icons.play_circle_outline), findsOneWidget);
    await tester.pump(const Duration(seconds: 1));
    await tester.pump();
    expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
    expect(find.text('S'), findsAtLeastNWidgets(2));
    expect(find.text(' 3'), findsAtLeastNWidgets(2));

    var opened = false;
    await tester.pumpWidget(ChangeNotifierProvider.value(
      value: state,
      child: MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 200,
            height: 200,
            child: w.PostTile(post: image, onTap: () => opened = true),
          ),
        ),
      ),
    ));
    await tester.tap(find.byType(w.PostTile));
    expect(opened, isTrue);
  });

  testWidgets('PostGrid lays out tiles and opens posts', (tester) async {
    tester.view.physicalSize = const Size(800, 600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    final state = await makeState();
    final posts = [Post.fromV2(postJson(1)), Post.fromV2(postJson(2))];
    Post? opened;
    await tester.pumpWidget(ChangeNotifierProvider.value(
      value: state,
      child: MaterialApp(
        home: Scaffold(
          body: w.PostGrid(posts: posts, onOpen: (p) => opened = p),
        ),
      ),
    ));
    await tester.pump();
    expect(find.byType(w.PostTile), findsNWidgets(2));
    await tester.tap(find.byType(w.PostTile).first);
    expect(opened?.id, 1);
  });

  testWidgets('TagChip strips underscores and taps', (tester) async {
    var tapped = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: w.TagChip(
            name: 'cat_ears',
            category: 0,
            onTap: () => tapped = true),
      ),
    ));
    expect(find.text('cat ears'), findsOneWidget);
    await tester.tap(find.text('cat ears'));
    expect(tapped, isTrue);
  });

  testWidgets('SearchBar submits and clears', (tester) async {
    final submitted = <String>[];
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: w.SearchBar(
            hint: 'hint', initial: 'fox', onSubmit: submitted.add),
      ),
    ));
    await tester.enterText(find.byType(TextField), 'wolf');
    await tester.testTextInput.receiveAction(TextInputAction.search);
    expect(submitted, ['wolf']);

    await tester.tap(find.byIcon(Icons.clear));
    expect(submitted, ['wolf', '']);
    expect(tester.widget<TextField>(find.byType(TextField)).controller?.text,
        '');
  });

  test('formatBytes covers each unit', () {
    expect(w.formatBytes(5), '5 B');
    expect(w.formatBytes(2048), '2.0 KB');
    expect(w.formatBytes(3 << 20), '3.0 MB');
    expect(w.formatBytes(2 << 30), '2.0 GB');
  });

  testWidgets('imagePlaceholder switches on progress', (tester) async {
    late BuildContext ctx;
    await tester.pumpWidget(MaterialApp(
        home: Builder(builder: (c) {
      ctx = c;
      return const SizedBox();
    })));
    const child = Text('x');
    expect(w.imagePlaceholder(ctx, child, null), same(child));
    final progress = ImageChunkEvent(
        cumulativeBytesLoaded: 1, expectedTotalBytes: 2);
    expect(w.imagePlaceholder(ctx, child, progress), isA<Container>());
  });
}
