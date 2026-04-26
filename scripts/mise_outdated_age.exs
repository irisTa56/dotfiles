#!/usr/bin/env elixir

defmodule MiseOutdatedAge do
  def main do
    case System.cmd("mise", ["outdated", "--json"]) do
      {out, 0} ->
        outdated = JSON.decode!(out)

        if outdated == %{} do
          IO.puts("All tools are up to date.")
        else
          render(outdated)
        end

      {err, code} ->
        msg = String.trim(err)
        IO.puts(:stderr, if(msg == "", do: "mise outdated failed (exit #{code})", else: msg))
        System.halt(code)
    end
  end

  defp render(outdated) do
    rows =
      Enum.map(outdated, fn {name, info} ->
        cur = Map.get(info, "current", "?")
        lat = Map.get(info, "latest", "?")
        {cur_d, lat_d} = release_dates(name, cur, lat)
        [name, Map.get(info, "requested", "?"), cur, cur_d, lat, lat_d]
      end)

    hdrs = ["Tool", "Requested", "Current", "Released", "Latest", "Released"]

    widths =
      [hdrs | rows]
      |> Enum.zip_with(fn col -> col |> Enum.map(&String.length/1) |> Enum.max() end)

    print_row = fn cells ->
      cells
      |> Enum.zip_with(widths, &String.pad_trailing/2)
      |> Enum.join("  ")
      |> IO.puts()
    end

    print_row.(hdrs)
    print_row.(Enum.map(widths, &String.duplicate("-", &1)))
    Enum.each(rows, print_row)
  end

  defp release_dates(tool, current, latest) do
    case System.cmd("mise", ["ls-remote", tool, "--json"]) do
      {out, 0} ->
        vmap =
          out
          |> JSON.decode!()
          |> Map.new(fn v -> {Map.get(v, "version"), Map.get(v, "created_at")} end)

        {fmt_date(Map.get(vmap, current)), fmt_date(Map.get(vmap, latest))}

      _ ->
        {"-", "-"}
    end
  rescue
    _ -> {"-", "-"}
  end

  defp fmt_date(iso) when iso in [nil, ""], do: "-"

  defp fmt_date(iso) do
    case DateTime.from_iso8601(iso) do
      {:ok, dt, _} ->
        days = div(DateTime.diff(DateTime.utc_now(), dt), 86_400)
        label = if days < 1, do: "today", else: "#{days}d ago"
        "#{Calendar.strftime(dt, "%Y-%m-%d")} (#{label})"

      _ ->
        String.slice(iso, 0, 10)
    end
  end
end

MiseOutdatedAge.main()
