fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "ScreenAsk");
        res.set("FileDescription", "ScreenAsk — Instant AI Screenshot Feedback");
        res.set("LegalCopyright", "Copyright (c) 2026 BB");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Failed to embed Windows icon resource: {e}");
        }
    }
}
