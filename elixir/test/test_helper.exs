# A git hook running this suite sets GIT_DIR and the like for its own
# repository, which would turn every git call here, and the code's, toward
# that one.
for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: System.delete_env(name)

ExUnit.start()
