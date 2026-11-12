import 'package:flutter_test/flutter_test.dart';
import 'package:self26/version_utils.dart';

void main() {
  Version? v(String? input) => Version.parse(input);

  test('parse basic versions', () {
    expect(v('1.2.3')!.core, [1, 2, 3]);
    expect(v('v1.2.3')!.core, [1, 2, 3]);
    expect(v('V1.2.3')!.core, [1, 2, 3]);
    expect(v('vv1.2.3')!.core, [1, 2, 3]);
    expect(v(' 1.2.3 ')!.core, [1, 2, 3]);
    expect(v('0.0.0')!.core, [0, 0, 0]);
    expect(v('10.20.30')!.core, [10, 20, 30]);
  });

  test('parse rejects invalid input', () {
    expect(v(null), isNull);
    expect(v(''), isNull);
    expect(v('   '), isNull);
    expect(v('1.2'), isNull);
    expect(v('1.2.3.4'), isNull);
    expect(v('1.2.x'), isNull);
    expect(v('a.b.c'), isNull);
    expect(v('01.2.3'), isNull);
    expect(v('1.02.3'), isNull);
    expect(v('1.2.-3'), isNull);
    expect(v('v'), isNull);
  });

  test('parse build metadata is stripped', () {
    final parsed = v('1.2.3+build.5');
    expect(parsed!.core, [1, 2, 3]);
    expect(parsed.pre, isNull);
    expect(parsed.toString(), '1.2.3');
  });

  test('parse prerelease', () {
    final parsed = v('1.2.3-rc.1');
    expect(parsed!.pre, ['rc', '1']);
    expect(parsed.toString(), '1.2.3-rc.1');

    expect(v('1.2.3-'), isNull);
    expect(v('1.2.3-a..b'), isNull);
    expect(v('1.2.3-äö'), isNull);
    expect(v('1.2.3-01'), isNull);
    expect(v('1.2.3-0'), isNotNull);
    expect(v('1.2.3-alpha-beta'), isNotNull);
  });

  test('compareTo core components', () {
    expect(v('1.0.0')!.compareTo(v('2.0.0')!), lessThan(0));
    expect(v('1.0.0')!.compareTo(v('1.1.0')!), lessThan(0));
    expect(v('1.0.1')!.compareTo(v('1.0.0')!), greaterThan(0));
    expect(v('1.0.0')!.compareTo(v('1.0.0')!), 0);
  });

  test('compareTo release vs prerelease', () {
    expect(v('1.0.0')!.compareTo(v('1.0.0-rc')!), greaterThan(0));
    expect(v('1.0.0-rc')!.compareTo(v('1.0.0')!), lessThan(0));
    expect(v('1.0.0-rc')!.compareTo(v('1.0.0-rc')!), 0);
  });

  test('compareTo prerelease identifiers', () {
    expect(v('1.0.0-2')!.compareTo(v('1.0.0-10')!), lessThan(0));
    expect(v('1.0.0-10')!.compareTo(v('1.0.0-2')!), greaterThan(0));
    expect(v('1.0.0-1')!.compareTo(v('1.0.0-alpha')!), lessThan(0));
    expect(v('1.0.0-alpha')!.compareTo(v('1.0.0-1')!), greaterThan(0));
    expect(v('1.0.0-alpha')!.compareTo(v('1.0.0-beta')!), lessThan(0));
    expect(v('1.0.0-rc')!.compareTo(v('1.0.0-rc.1')!), lessThan(0));
    expect(v('1.0.0-99999999999999999999')!
        .compareTo(v('1.0.0-100000000000000000000')!), lessThan(0));
  });

  test('equality and hashCode', () {
    expect(v('1.2.3'), v('1.2.3'));
    expect(v('1.2.3'), isNot(v('1.2.4')));
    expect(v('1.2.3-rc'), v('1.2.3-rc'));
    expect(v('1.2.3'), isNot(v('1.2.3-rc')));
    expect(v('1.2.3-rc'), isNot(v('1.2.3')));
    expect(v('1.2.3-rc.1'), isNot(v('1.2.3-rc')));
    expect(v('1.2.3') == Object(), isFalse);
    expect(v('1.2.3')!.hashCode, v('v1.2.3')!.hashCode);
    expect(v('1.2.3-rc')!.hashCode, isA<int>());
  });

  test('toString', () {
    expect(v('1.2.3').toString(), '1.2.3');
    expect(v('1.2.3-rc.1').toString(), '1.2.3-rc.1');
    expect(Version(core: const [0, 1, 0]).toString(), '0.1.0');
  });
}
