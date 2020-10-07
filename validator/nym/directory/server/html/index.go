package html

import (
	"time"

	"github.com/jessevdk/go-assets"
)

var _Assets87ef411210acb6cc2ef5dc23af8ae586f1a46c19 = "<!DOCTYPE html>\n<html lang=\"en\">\n\n<head>\n    <title>Chat Example</title>\n    <script type=\"text/javascript\">\n        window.onload = function () {\n            var conn;\n            var msg = document.getElementById(\"msg\");\n            var log = document.getElementById(\"log\");\n\n            function appendLog(item) {\n                var doScroll = log.scrollTop > log.scrollHeight - log.clientHeight - 1;\n                log.appendChild(item);\n                if (doScroll) {\n                    log.scrollTop = log.scrollHeight - log.clientHeight;\n                }\n            }\n\n            document.getElementById(\"form\").onsubmit = function () {\n                if (!conn) {\n                    return false;\n                }\n                if (!msg.value) {\n                    return false;\n                }\n                conn.send(msg.value);\n                msg.value = \"\";\n                return false;\n            };\n\n            if (window[\"WebSocket\"]) {\n                if (location.hostname === \"localhost\" || location.hostname === \"127.0.0.1\") {\n                    conn = new WebSocket(\"ws://\" + document.location.host + \"/ws\");\n                }else {\n                    conn = new WebSocket(\"wss://\" + document.location.host + \"/ws\");\n                }\n                conn.onclose = function (evt) {\n                    var item = document.createElement(\"div\");\n                    item.innerHTML = \"<b>Connection closed.</b>\";\n                    appendLog(item);\n                };\n                conn.onmessage = function (evt) {\n                    var messages = evt.data.split('\\n');\n                    for (var i = 0; i < messages.length; i++) {\n                        var item = document.createElement(\"div\");\n                        item.innerText = messages[i];\n                        appendLog(item);\n                    }\n                };\n            } else {\n                var item = document.createElement(\"div\");\n                item.innerHTML = \"<b>Your browser does not support WebSockets.</b>\";\n                appendLog(item);\n            }\n        };\n    </script>\n    <style type=\"text/css\">\n        html {\n            overflow: hidden;\n        }\n\n        body {\n            overflow: hidden;\n            padding: 0;\n            margin: 0;\n            width: 100%;\n            height: 100%;\n            background: gray;\n        }\n\n        #log {\n            background: white;\n            margin: 0;\n            padding: 0.5em 0.5em 0.5em 0.5em;\n            position: absolute;\n            top: 0.5em;\n            left: 0.5em;\n            right: 0.5em;\n            bottom: 3em;\n            overflow: auto;\n        }\n\n        #form {\n            padding: 0 0.5em 0 0.5em;\n            margin: 0;\n            position: absolute;\n            bottom: 1em;\n            left: 0px;\n            width: 100%;\n            overflow: hidden;\n        }\n    </style>\n</head>\n\n<body>\n    <div id=\"log\"></div>\n    <form id=\"form\">\n        <input type=\"submit\" value=\"Send\" />\n        <input type=\"text\" id=\"msg\" size=\"64\" />\n    </form>\n</body>\n\n</html>"

// Assets returns go-assets FileSystem
var Assets = assets.NewFileSystem(map[string][]string{"/": []string{"server"}, "/server": []string{"html"}, "/server/html": []string{"index.html"}}, map[string]*assets.File{
	"/": &assets.File{
		Path:     "/",
		FileMode: 0x800001fd,
		Mtime:    time.Unix(1569951733, 1569951733000002230),
		Data:     nil,
	}, "/server": &assets.File{
		Path:     "/server",
		FileMode: 0x800001fd,
		Mtime:    time.Unix(1569951733, 1569951733004002245),
		Data:     nil,
	}, "/server/html": &assets.File{
		Path:     "/server/html",
		FileMode: 0x800001fd,
		Mtime:    time.Unix(1569951733, 1569951733004002245),
		Data:     nil,
	}, "/server/html/index.html": &assets.File{
		Path:     "/server/html/index.html",
		FileMode: 0x1b4,
		Mtime:    time.Unix(1569951814, 1569951814816302011),
		Data:     []byte(_Assets87ef411210acb6cc2ef5dc23af8ae586f1a46c19),
	}}, "")
