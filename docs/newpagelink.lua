-- newpagelink.lua

function Link(link)
  -- Open links in a new tab, except for those that point to a different section
  -- on the same page.
  if not link.target:match("^#") then
    link.attributes["target"] = "_blank"
  end
  return link
end
