# Cask da versão publicada 0.2 (SwiftUI). Copie para um tap (ex.:
# homebrew-falcaosl/Casks/multidroid.rb) ou instale direto do clone:
#
#   brew install --cask ./packaging/homebrew/Casks/multidroid.rb
#
# Atualize version/sha256 após cada release notarizada:
#   packaging/homebrew/update-cask.sh v0.2
cask "multidroid" do
  version "0.2"
  sha256 "9d671ec73e8ddddfcd285468e0280f2c1ce3f157effedf114623ba1059dcb92c"

  url "https://github.com/iramarfalcao/MultiDroid/releases/download/v#{version}/MultiDroid-#{version}.dmg"
  name "MultiDroid"
  desc "Several Android devices on one Mac desk"
  homepage "https://multidroid.falcaosl.com"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :tahoe"
  depends_on arch: :arm64

  app "MultiDroid.app"

  zap trash: [
    "~/Library/Application Support/MultiDroid",
    "~/Library/Preferences/com.falcaosl.multidroid.plist",
    "~/Library/Caches/com.falcaosl.multidroid",
  ]
end
