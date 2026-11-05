defmodule Dotfiles.WithNewTerminal do
  @usage """
  Usage: with-new-terminal <script.sh> [--cwd <dir>] [--title <title>]

  Runs a shell script in a new Terminal window, leaving out its comment and
  blank lines.

    --cwd <dir>      the directory the script runs in
    --title <title>  the window's title; the script's file name by default
    --from <dir>     the directory a relative script path is read from; the
                     shell function passes the one it was called from, since
                     the task itself runs from the Mix project
  """

  def main(argv) do
    case OptionParser.parse(argv, strict: [cwd: :string, title: :string, from: :string]) do
      {opts, [script_path], []} ->
        title = opts[:title] || Path.basename(script_path)
        run(Path.expand(script_path, opts[:from] || File.cwd!()), opts[:cwd], title)

      _ ->
        die(@usage, 1)
    end
  end

  defp run(script_path, cwd, title) do
    body =
      script_path
      |> File.read!()
      |> String.split("\n")
      |> Enum.reject(&comment_or_blank?/1)
      |> Enum.join("\n")

    body =
      case cwd do
        nil -> body
        dir -> "cd #{dir}\n" <> body
      end

    body = "setopt no_bang_hist && clear\n" <> body

    script = escape(String.trim_trailing(body))
    title = escape(title)

    applescript = """
    tell application "Terminal"
      do script "#{script}"
      set custom title of front window to "#{title}"
    end tell
    """

    case Dotfiles.Command.cmd("osascript", ["-e", applescript]) do
      {_, 0} -> :ok
      {_, code} -> die("osascript failed (exit #{code})", code)
    end
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

  # `exit({:shutdown, code})` rather than halting, so elixir's own CLI ends the
  # run with that status once its output is flushed, and a test can catch it.
  defp die(message, code) do
    IO.puts(:stderr, message)
    exit({:shutdown, code})
  end
end
