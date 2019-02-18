"use strict";

const list = window.document.querySelectorAll(".timestamp");

for (let timestamp of list) {
    const m = moment(timestamp.innerHTML);
    if (m.isValid()) {
        timestamp.innerHTML = m.calendar();
    }
}
