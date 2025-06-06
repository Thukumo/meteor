import 'package:client/room.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Meteor クライアントアプリ',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color.fromARGB(255, 106, 255, 143)),
      ),
      onGenerateRoute: (settings) {
        if (!kIsWeb) {
          // Web以外はクエリパラメータを考慮せず、通常の画面遷移のみ
          if (settings.name == '/room' && settings.arguments is String) {
            final roomName = settings.arguments as String;
            return MaterialPageRoute(
              builder: (context) => Room(roomName: roomName),
              settings: settings,
            );
          }
          return MaterialPageRoute(
            builder: (context) => MyHomePage(),
            settings: settings,
          );
        }
        // Webの場合のみクエリパラメータを考慮
        final uri = Uri.parse(settings.name ?? '');
        if (uri.path == '/room' && uri.queryParameters['room'] != null) {
          final roomName = uri.queryParameters['room']!;
          return MaterialPageRoute(
            builder: (context) => Room(roomName: roomName),
            settings: settings,
          );
        }
        // デフォルトはホーム
        return MaterialPageRoute(
          builder: (context) => MyHomePage(),
          settings: settings,
        );
      },
      initialRoute: kIsWeb && Uri.base.path == '/room' && Uri.base.queryParameters['room'] != null
          ? '/room?room=${Uri.base.queryParameters['room']}'
          : '/',
      home: MyHomePage(),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({super.key});

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  String room = "";
  @override
  void initState() {
    super.initState();
    if (kIsWeb) {
      // Webの場合、ルーム名をURLのクエリパラメータから取得
      final uri = Uri.base;
      room = uri.queryParameters['room'] ?? "";
      if (room.isNotEmpty) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          // pushNamedでURLを正しく書き換えて遷移
          Navigator.pushNamed(
            context,
            '/room?room=$room',
          );
        });
      }
    }
  }
  void connect() {
    if (room.isNotEmpty) {
    // pushNamedでURLを書き換えて遷移
    Navigator.pushNamed(
      context,
      '/room?room=$room',
    );
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text("ルーム名を入力してください"))
      );
    }
  }
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text("Meteor"),
      ),
      body: Column(children: [
        Align(
          alignment: Alignment.topCenter,
          child: SizedBox(
            width: MediaQuery.of(context).size.width * 0.8,
            child: TextField(
              decoration: InputDecoration(
                border: OutlineInputBorder(),
                hintText: "接続先のルーム名を入力"
              ),
              onChanged: (value) {
                setState(() {
                  room = value;
                });
              },
              onSubmitted: (_) => connect(),
              autofocus: true,
            ),
          ),
        ),
        ElevatedButton(
          onPressed: () => connect(),
          style: ElevatedButton.styleFrom(
            backgroundColor: Color.fromARGB(255, 106, 255, 143),
            foregroundColor: Colors.black,
          ),
          child: Text("接続"),
        )
      ]),
    );
  }
}
