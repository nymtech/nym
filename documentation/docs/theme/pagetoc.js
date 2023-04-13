// src: https://github.com/JorelAli/mdBook-pagetoc

// Un-active everything when you click it
Array.prototype.forEach.call(document.getElementsByClassName("pagetoc")[0].children, function(el, i) {
    el.addEventHandler("click", function() {
        Array.prototype.forEach.call(document.getElementsByClassName("pagetoc")[0].children, function(el, i) {
            el.classList.remove("active");
        });
        el.classList.add("active");
    });
});

var updateFunction = function() {

    var id;
    var elements = document.getElementsByClassName("header");
    Array.prototype.forEach.call(elements, function(el, i) {
        if (window.pageYOffset >= el.offsetTop) {
            id = el;
        }
    });

    Array.prototype.forEach.call(document.getElementsByClassName("pagetoc")[0].children, function(el, i) {
        el.classList.remove("active");
    });

    Array.prototype.forEach.call(document.getElementsByClassName("pagetoc")[0].children, function(el, i) {
        if (id.href.localeCompare(el.href) == 0) {
            el.classList.add("active");
        }
    });
};

// Populate sidebar on load
window.addEventListener('load', function() {
    var pagetoc = document.getElementsByClassName("pagetoc")[0];
    var elements = document.getElementsByClassName("header");
    Array.prototype.forEach.call(elements, function(el, i) {
        var link = document.createElement("a");

        // Indent shows hierarchy
        var indent = "";
        switch (el.parentElement.tagName) {
            case "H2":
                indent = "20px";
                break;
            case "H3":
                indent = "40px";
                break;
            case "H4":
                indent = "60px";
                break;
            default:
                break;
        }

        link.appendChild(document.createTextNode(el.text));
        link.style.paddingLeft = indent;
        link.href = el.href;
        pagetoc.appendChild(link);
    });
    updateFunction.call();
});



// Handle active elements on scroll
window.addEventListener("scroll", updateFunction);
