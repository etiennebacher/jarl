// Adds a search box above the table of rules, filtering on all columns.
document$.subscribe(function () {
  var container = document.getElementById("rules-table");
  if (!container || container.querySelector("input.rules-filter")) return;

  var table = container.querySelector("table");
  if (!table) return;

  var input = document.createElement("input");
  input.type = "search";
  input.className = "rules-filter";
  input.placeholder = "Filter rules (name, group, fix, conditions)";
  input.setAttribute("aria-label", "Filter rules");
  container.insertBefore(input, container.firstChild);

  input.addEventListener("input", function () {
    var query = input.value.trim().toLowerCase();
    table.querySelectorAll("tbody tr").forEach(function (row) {
      row.hidden = query !== "" && !row.textContent.toLowerCase().includes(query);
    });
  });
});
