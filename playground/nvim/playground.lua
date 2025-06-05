vim.env.LAZY_STDPATH = ".playground"
load(vim.fn.system("curl -s https://raw.githubusercontent.com/folke/lazy.nvim/main/bootstrap.lua"))()

vim.lsp.config("lsp-tutorial", {
    cmd = { "rust-lsp-tutorial" },
})
vim.diagnostic.config({ virtual_text = true })
vim.lsp.set_log_level(vim.log.levels.DEBUG)

require("lazy.minit").repro({
    defaults = {
        lazy = true,
    },
    spec = {
        {
            "neovim/nvim-lspconfig",
            lazy = false,
            init = function()
                vim.lsp.enable({
                    "lsp-tutorial",
                    "lua_ls",
                })
            end,
        },
        {
            "saghen/blink.cmp",
            version = "*",
            event = { "InsertEnter" },
            opts = {},
        },
    },
})
