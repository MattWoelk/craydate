# Set Up
1. rustup target add armv7a-none-eabi
1. Install playdate SDK: https://play.date/dev/
1. Set PLAYDATE_SDK_PATH env var to point at the SDK install dir
    * Or set it in [env] section of Cargo config file.
1. Install clang: https://releases.llvm.org/download.html
    * For windows look for a .exe file on the GitHub release page. It doesn't say clang in the name but it's there.
