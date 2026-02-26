// Force dark mode on first visit (before system preference kicks in)
if (!localStorage.getItem("data-md-color-scheme")) {
  document.body.setAttribute("data-md-color-scheme", "slate");
  document.body.setAttribute("data-md-color-primary", "deep-purple");
  document.body.setAttribute("data-md-color-accent", "amber");
}
