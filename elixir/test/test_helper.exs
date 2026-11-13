# A unit test runs no external command: each answers through the handler the test
# installs in Dotfiles.FakeCommand, and one the test did not install fails it. An
# integration test puts back the real implementation for itself.
Application.put_env(:dotfiles, :command, Dotfiles.FakeCommand)

# A git hook running this suite sets GIT_DIR and the like for its own repository,
# which would turn every git call a test makes toward that one: a `git init` there
# marks the repository bare. Clearing them here covers every test, so none has to
# remember to.
for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: System.delete_env(name)

ExUnit.start()
