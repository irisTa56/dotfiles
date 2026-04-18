#!/usr/bin/env elixir

defmodule Helper do
  def run(script_path, cwd, title) do
    body =
      script_path
      |> File.read!()
      |> String.split("\n")
      |> Enum.reject(&comment_or_blank?/1)
      |> Enum.join("\n")

    body = "setopt no_bang_hist && clear\n" <> body

    body =
      case cwd do
        nil -> body
        dir -> "cd #{dir}\n" <> body
      end

    script = escape(String.trim_trailing(body))
    title = escape(title)

    {_, 0} =
      System.shell("""
      osascript << 'APPLESCRIPT'
      tell application "Terminal"
        do script "#{script}"
        set custom title of front window to "#{title}"
      end tell
      APPLESCRIPT
      """)
  end

  defp comment_or_blank?(line) do
    trimmed = String.trim(line)
    trimmed == "" or String.starts_with?(trimmed, "#")
  end

  defp escape(s) do
    s
    |> String.replace("\\", "\\\\")
    |> String.replace("\"", "\\\"")
    |> String.replace("\n", "\\n")
  end
end

{opts, args, _} =
  OptionParser.parse(System.argv(), strict: [cwd: :string, title: :string])

case args do
  [script_path] ->
    title = opts[:title] || Path.basename(script_path)
    Helper.run(script_path, opts[:cwd], title)

  _ ->
    IO.puts(
      :stderr,
      "Usage: with_new_terminal.exs <script.sh> [--cwd <dir>] [--title <title>]"
    )

    System.halt(1)
end
