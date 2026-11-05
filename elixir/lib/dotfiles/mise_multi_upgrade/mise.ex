# Every call Dotfiles.MiseMultiUpgrade makes to mise goes through this, so a test can
# answer in mise's place.
defmodule Dotfiles.MiseMultiUpgrade.Mise do
  @callback cmd(args :: [String.t()], opts :: keyword()) ::
              {Collectable.t(), non_neg_integer()}
end
