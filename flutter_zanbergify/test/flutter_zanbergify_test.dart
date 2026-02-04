import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_zanbergify/flutter_zanbergify.dart';
import 'package:flutter_zanbergify/flutter_zanbergify_platform_interface.dart';
import 'package:flutter_zanbergify/flutter_zanbergify_method_channel.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';

class MockFlutterZanbergifyPlatform
    with MockPlatformInterfaceMixin
    implements FlutterZanbergifyPlatform {

  @override
  Future<String?> getPlatformVersion() => Future.value('42');
}

void main() {
  final FlutterZanbergifyPlatform initialPlatform = FlutterZanbergifyPlatform.instance;

  test('$MethodChannelFlutterZanbergify is the default instance', () {
    expect(initialPlatform, isInstanceOf<MethodChannelFlutterZanbergify>());
  });

  test('getPlatformVersion', () async {
    FlutterZanbergify flutterZanbergifyPlugin = FlutterZanbergify();
    MockFlutterZanbergifyPlatform fakePlatform = MockFlutterZanbergifyPlatform();
    FlutterZanbergifyPlatform.instance = fakePlatform;

    expect(await flutterZanbergifyPlugin.getPlatformVersion(), '42');
  });
}
