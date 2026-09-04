-- linkify-github-refs.lua
--
-- Turn plain GitHub references in the changelog into links at render time, so
-- that the Markdown source stays readable:
--
--   (#650)              ->  ([#650](https://github.com/etiennebacher/jarl/issues/650))
--   (#459, @some-user)  ->  ([#459](...), [@some-user](https://github.com/some-user))
--
-- References inside code (`@examples`, `%notin%`, `# nolint`) and inside
-- existing links are left alone.

local repo_url = "https://github.com/etiennebacher/jarl"

local function is_changelog()
  -- Quarto renders a temporary copy of the file, so the original path has to
  -- come from Quarto itself when this runs as part of a Quarto render.
  local input
  if quarto ~= nil then
    input = quarto.doc.input_file
  else
    input = PANDOC_STATE.input_files[1]
  end
  return input ~= nil and input:match("changelog%.md$") ~= nil
end

-- Only linkify a reference that starts a "word", to avoid touching things like
-- URL fragments or email addresses.
local function starts_word(text, start)
  return start == 1 or text:sub(start - 1, start - 1):match("[%w_/@#-]") == nil
end

local function reference_link(sigil, name)
  if sigil == "#" then
    -- GitHub redirects issue URLs to pull requests, so this works for both.
    return pandoc.Link("#" .. name, repo_url .. "/issues/" .. name)
  end
  return pandoc.Link("@" .. name, "https://github.com/" .. name)
end

local function linkify(text)
  local inlines = pandoc.Inlines({})
  local pos = 1
  local found = false

  while pos <= #text do
    local start, stop, sigil, name = text:find("([#@])([%w][%w-]*)", pos)
    if start == nil then
      break
    end

    -- Issue references are numeric, user names are not purely numeric.
    local valid = starts_word(text, start) and
      (sigil == "#") == (name:match("^%d+$") ~= nil)

    if valid then
      found = true
      if start > pos then
        inlines:insert(pandoc.Str(text:sub(pos, start - 1)))
      end
      inlines:insert(reference_link(sigil, name))
      pos = stop + 1
    else
      -- Move past this candidate and keep looking in the rest of the string.
      inlines:insert(pandoc.Str(text:sub(pos, stop)))
      pos = stop + 1
    end
  end

  if not found then
    return nil
  end
  if pos <= #text then
    inlines:insert(pandoc.Str(text:sub(pos)))
  end
  return inlines
end

local filters = {}

if is_changelog() then
  filters = {
    {
      traverse = "topdown",
      -- Don't rewrite the text of links that are already in the source.
      Link = function(link)
        return link, false
      end,
      Str = function(str)
        local inlines = linkify(str.text)
        if inlines == nil then
          return nil
        end
        -- `false` keeps the new links from being scanned for references again.
        return inlines, false
      end,
    },
  }
end

return filters
