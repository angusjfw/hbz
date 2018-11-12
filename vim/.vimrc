" Plugins
call plug#begin()
Plug 'Shougo/vimproc.vim', {'do' : 'make'}
Plug 'w0rp/ale'
Plug 'chriskempson/base16-vim'
Plug 'scrooloose/nerdtree'
Plug 'christoomey/vim-tmux-navigator'
Plug 'mileszs/ack.vim'
Plug 'junegunn/vim-emoji'
Plug 'junegunn/fzf', { 'dir': '~/.fzf', 'do': './install --all' }
Plug 'junegunn/goyo.vim'
Plug 'junegunn/rainbow_parentheses.vim'
Plug 'sheerun/vim-polyglot'
Plug 'pangloss/vim-javascript'
Plug 'hail2u/vim-css3-syntax'
Plug 'ap/vim-css-color'
Plug 'mxw/vim-jsx'
Plug 'martinda/Jenkinsfile-vim-syntax'
Plug 'JamshedVesuna/vim-markdown-preview'
Plug 'othree/html5.vim'
Plug 'Quramy/tsuquyomi'
Plug 'leafgarland/typescript-vim'
call plug#end()

" Keybinds
let mapleader = ','
map <C-f> :FZF<CR>
map <C-g> :Goyo<CR>
map <C-p> :%!python -m json.tool<CR>
if executable('ag')
  let g:ackprg = 'ag --vimgrep'
endif
map <C-s> :Ack!<Space>
map <C-y> r<C-v>u2713
map <C-n> r<C-v>u2717
map <C-h> <C-w>h
map <C-j> <C-w>j
map <C-k> <C-w>k
map <C-l> <C-w>l
noremap <Up> <NOP>
noremap <Down> <NOP>
noremap <Left> <NOP>
noremap <Right> <NOP>

" General
filetype off
filetype plugin indent on
syntax on
set noshowmode
set timeoutlen=420
set autoindent
set autoread                                                  " reload files when changed on disk, i.e. via `git checkout`
set backspace=2                                               " Fix broken backspace in some setups
set backupcopy=yes                                            " see :help crontab
set clipboard=unnamedplus                                     " yank and paste with the X11's CLIPBOARD
set directory-=.                                              " don't store swapfiles in the current directory
set encoding=utf-8
set hlsearch                                                  " highlight search results
set ignorecase                                                " case-insensitive search
set incsearch                                                 " search as you type
set laststatus=2                                              " always show statusline
set list                                                      " show trailing whitespace
set listchars=tab:▸\ ,trail:▫,extends:>,precedes:<
set fillchars+=vert:\ 
set number                                                    " show line numbers
set ruler                                                     " show where you are
set scrolloff=3                                               " show context above/below cursorline
set showcmd
set showmode
set smartcase                                                 " case-sensitive search if any caps
set wildignore=log/**,node_modules/**,target/**,tmp/**,*.rbc
set wildmenu                                                  " show a navigable menu for tab completion
set wildmode=longest,list,full
set mouse=a
set textwidth=79
set formatoptions-=t                                          " please no wrap!
set colorcolumn=+1
set undodir=~/.vim/undo
set undofile
set undolevels=1000
set undoreload=10000                                          " maximum number lines to save for undo on a
set completeopt-=preview
set exrc                                                      " source local .vimrc files
set secure                                                    " do not allow local .vimrc to perform insecure ops
set shiftwidth=2                                              " normal mode indentation commands use 2 spaces
set softtabstop=2                                             " insert mode tab and backspace use 4 spaces
set tabstop=4                                                 " actual tabs occupy 4 characters
set expandtab                                                 " expand tabs to spaces

" allow toggling between local and default mode
function TabToggle()
  if &expandtab
    set shiftwidth=4
    set softtabstop=0
    set noexpandtab
  else
    set shiftwidth=2
    set softtabstop=2
    set expandtab
  endif
endfunction
nmap <F9> mz:execute TabToggle()<CR>'z

" Hacks
map q: <Nop>                                                  " prevent entering Ex mode
nnoremap Q <nop>                                              " prevent entering Ex mode
nnoremap <CR> :noh<CR><CR>                                    " enter clears search highlight
vnoremap p "_dP                                               " Don't copy the contents of an overwritten selection.
autocmd FileType javascript set tabstop=2|set shiftwidth=2|set expandtab

" Tmux
autocmd VimEnter * nnoremap <silent> <c-j> :TmuxNavigateDown<cr>:redraw!<cr>
if exists('$TMUX')                                            " Support resizing in tmux
  let &t_SI = "\<Esc>Ptmux;\<Esc>\<Esc>]50;CursorShape=1\x7\<Esc>\\"
  let &t_EI = "\<Esc>Ptmux;\<Esc>\<Esc>]50;CursorShape=0\x7\<Esc>\\"
else
  let &t_SI = "\<Esc>]50;CursorShape=1\x7"
  let &t_EI = "\<Esc>]50;CursorShape=0\x7"
endif

" Plugin settings
let vim_markdown_preview_github=1
let vim_markdown_preview_browser='chromium'
let vim_markdown_preview_use_xdg_open=1
" NERDTree
let NERDTreeMapHelp='<f1>'
nmap <leader>d :NERDTreeToggle<CR>
let g:NERDSpaceDelims=1
let g:NERDTreeMinimalUI=2
let g:NERDTreeWinSize=25
let NERDTreeIgnore=['\.pyc$']
let NERDTreeIgnore=['\.swp$']
let NERDTreeShowHidden=1

" linting
let g:tsuquyomi_disable_quickfix = 1

" tsuquyomi
autocmd FileType typescript nmap <buffer> <Leader>t : <C-u>echo tsuquyomi#hint()<CR>

" Theme
set termguicolors
colorscheme base16-eighties

" goyo
let g:goyo_width = 100
