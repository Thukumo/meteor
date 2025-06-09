import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:web/web.dart' as web;

import 'package:web_socket_channel/web_socket_channel.dart';

class Room extends StatefulWidget {
  final String roomName;

  const Room({super.key, required this.roomName});

  @override
  State<Room> createState() => _RoomState();
}
class _RoomState extends State<Room> {
  final List<String> history = [];
  WebSocketChannel? channel;
  bool _loading = true;
  String? _error;
  final TextEditingController _controller = TextEditingController();
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    fetchHistory();
  }

  Future<void> fetchHistory() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final origin = web.window.location.origin;
      final apiUrl = '$origin/api/v1/room/${widget.roomName}/history';
      final response = await http.get(Uri.parse(apiUrl));
      if (response.statusCode == 200) {
        final List<dynamic> data = json.decode(response.body);
        setState(() {
          history.clear();
          history.addAll(data.map((item) => item.toString()));
          _loading = false;
        });
      } else {
        setState(() {
          _loading = false;
          _error = 'サーバーエラー: ${response.statusCode}';
        });
      }
    } catch (e) {
      setState(() {
        _loading = false;
        _error = '通信エラー: $e';
      });
    }
    // websocketの接続を開始
    final origin = web.window.location.origin;
    final wsScheme = origin.startsWith('https') ? 'wss' : 'ws';
    final wsOrigin = origin.replaceFirst(RegExp(r'^https?'), wsScheme);
    channel = WebSocketChannel.connect(
      Uri.parse('$wsOrigin/api/v1/room/${widget.roomName}/ws'),
    );
    channel!.stream.listen((message) {
      setState(() {
        history.add(message);
      });
    });
  }
  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose(); // 追加
    super.dispose();
    channel?.sink.close();
  }

  void sendMessage() {
    final text = _controller.text.trim();
    if (text.isNotEmpty && channel != null) {
      channel!.sink.add(text);
      _controller.clear();
      FocusScope.of(context).requestFocus(_focusNode); // フォーカスを戻す
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text("ルーム ${widget.roomName}"),
        actions: [
          IconButton(
            icon: Icon(Icons.link),
            tooltip: 'ルームリンクをコピー',
            onPressed: () async {
              final origin = web.window.location.origin;
              await Clipboard.setData(ClipboardData(text: '$origin/index.html#/room?room=${widget.roomName}'));
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('ルームへのリンクをコピーしました')),
                );
              }
            },
          ),
        ],
      ),
      body: Column(
        children: [
          Expanded(
            child: _loading
                ? Center(child: CircularProgressIndicator())
                : _error != null
                    ? Center(child: Text(_error!))
                    : history.isEmpty
                        ? Center(child: Text("履歴がありません"))
                        : ListView.separated(
                            itemCount: history.length,
                            separatorBuilder: (context, index) => Divider(height: 1),
                            itemBuilder: (context, index) {
                              return ListTile(
                                title: Text(history[index]),
                              );
                            },
                          ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _controller,
                    focusNode: _focusNode, // 追加
                    decoration: InputDecoration(
                      hintText: 'メッセージを入力',
                      border: OutlineInputBorder(),
                    ),
                    onSubmitted: (_) => sendMessage(),
                  ),
                ),
                SizedBox(width: 8),
                ElevatedButton(
                  onPressed: sendMessage,
                  child: Text('送信'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
