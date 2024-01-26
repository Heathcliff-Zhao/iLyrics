Set WshShell = createObject("WScript.Shell")
WshShell.run "cmd /c node E:\Study\Projectlearning\iLyrics\api\app.js", 0, False
WshShell.run "D:\ToolsForITunes\ilyrics&iTunes.bat",0
Set WshShell = Nothing