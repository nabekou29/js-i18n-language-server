vim.env.LAZY_STDPATH = ".repro_minimal"
load(vim.fn.system("curl -s https://raw.githubusercontent.com/folke/lazy.nvim/main/bootstrap.lua"))()

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

vim.opt.number = true
vim.opt.tabstop = 2
vim.opt.shiftwidth = 2
vim.opt.swapfile = false

vim.diagnostic.config({
    virtual_text = {},
    underline = true,
    update_in_insert = true,
})

vim.lsp.log.set_level("info")

vim.lsp.config("js_i18n_ls", {
    cmd = { "js-i18n-language-server" },
    filetypes = { "javascript", "typescript", "javascriptreact", "typescriptreact", "json" },
    root_markers = { "package.json", ".git" },
})
vim.lsp.enable({ "js_i18n_ls" })

vim.keymap.set("n", "gd", "<cmd>lua vim.lsp.buf.definition()<cr>")
vim.keymap.set("n", "gr", "<cmd>lua vim.lsp.buf.references()<cr>")
vim.keymap.set("n", "ga", "<cmd>lua vim.lsp.buf.code_action()<cr>")
vim.keymap.set("n", "K", "<cmd>lua vim.lsp.buf.hover()<cr>")

-- Register command
vim.api.nvim_create_user_command("LspRestart", function()
    vim.lsp.stop_client(vim.lsp.get_clients())
    vim.cmd("edit")
end, {})
