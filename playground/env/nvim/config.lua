vim.env.LAZY_STDPATH = ".repro_minimal"
load(vim.fn.system("curl -s https://raw.githubusercontent.com/folke/lazy.nvim/main/bootstrap.lua"))()

vim.opt.number = true
vim.opt.tabstop = 2
vim.opt.shiftwidth = 2
vim.opt.swapfile = false

require("lazy.minit").repro({
    spec = {
        { "windwp/nvim-autopairs", opts = {} },
        {
            "saghen/blink.cmp",
            version = "*",
            opts = {
                sources = {
                    default = { "lsp" },
                },
            },
            opts_extend = { "sources.default" },
        },
        {
            "nvim-treesitter/nvim-treesitter",
            config = function()
                require("nvim-treesitter.configs").setup({
                    auto_install = true,
                    ensure_installed = { "javascript" },
                    ignore_install = {},
                    highlight = {
                        enable = true,
                        additional_vim_regex_highlighting = false,
                    },
                })
            end,
        },
    },
})

vim.lsp.log.set_level("info")
vim.lsp.config("js_i18n_ls", {
    cmd = { "js-i18n-language-server" },
    filetypes = { "javascript", "typescript", "javascriptreact", "typescriptreact", "jons" },
    root_markers = { "package.json", ".git" },
})
vim.lsp.enable({ "js_i18n_ls" })

-- Register command
vim.api.nvim_create_user_command("LspRestart", function()
    vim.lsp.stop_client(vim.lsp.get_clients())
    vim.cmd("edit")
end, {})
