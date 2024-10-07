class MediaWidget extends HTMLElement {
    constructor() {
        super();
    }

    connectedCallback() {
        this.art_url_hash = null;
        this.child_img = document.createElement("img");
        this.appendChild(this.child_img);

        this.child_title = document.createElement("span");
        this.appendChild(this.child_title);
        
        this.child_prev = document.createElement("button");
        this.child_prev.textContent = "Prev";
        this.child_prev.addEventListener("click", event => {
            fetch(`/prev/${this.getAttribute("media-id")}`, { method: "POST" });
            event.stopPropagation();
        });
        this.appendChild(this.child_prev);

        this.child_seek_backward = document.createElement("button");
        this.child_seek_backward.textContent = "Seek Backward";
        this.child_seek_backward.addEventListener("click", event => {
            fetch(`/seek/${this.getAttribute("media-id")}/-10000000`, { method: "POST" });
            event.stopPropagation();
        });
        this.appendChild(this.child_seek_backward);

        this.child_seek_forward = document.createElement("button");
        this.child_seek_forward.textContent = "Seek Forward";
        this.child_seek_forward.addEventListener("click", event => {
            fetch(`/seek/${this.getAttribute("media-id")}/+10000000`, { method: "POST" });
            event.stopPropagation();
        });
        this.appendChild(this.child_seek_forward);

        this.child_next = document.createElement("button");
        this.child_next.textContent = "Next";
        this.child_next.addEventListener("click", event => {
            fetch(`/next/${this.getAttribute("media-id")}`, { method: "POST" });
            event.stopPropagation();
        });
        this.appendChild(this.child_next);

        this.update_progress = null;
        this.progress_base = 0;
        this.playback_rate = 1;
        this.child_progress = document.createElement("progress");
        this.appendChild(this.child_progress);

        this.eventSource = this.get_updates();

        this.addEventListener("click", () => {
            fetch(`/playpause/${this.getAttribute("media-id")}`, { method: "POST" });
        });
    }

    disconnectedCallback() {
        this.eventSource.close();
        cancelAnimationFrame(this.update_progress);
    }

    get_updates() {
        const eventSource = new EventSource(`/metadata/${this.getAttribute("media-id")}`);
        eventSource.addEventListener("update", event => {
            const data = JSON.parse(event.data);

            if (data.art_url_hash !== this.art_url_hash) {
                this.child_img.src = `/icon/${this.getAttribute("media-id")}/${data.art_url_hash}`;
                this.art_url_hash = data.art_url_hash;
            }

            this.child_title.textContent = data.title;

            this.child_progress.value = data.position;
            this.child_progress.max = data.length;

            if (data.can_go_prev) {
                this.child_prev.classList.remove("hidden");
            } else {
                this.child_prev.classList.add("hidden");
            }

            if (data.can_seek) {
                this.child_seek_backward.classList.remove("hidden");
                this.child_seek_forward.classList.remove("hidden");
            } else {
                this.child_seek_backward.classList.add("hidden");
                this.child_seek_forward.classList.add("hidden");
            }

            if (data.can_go_next) {
                this.child_next.classList.remove("hidden");
            } else {
                this.child_next.classList.add("hidden");
            }

            if (data.running && this.update_progress === null) {
                this.playback_rate = data.playback_rate ?? 1;
                this.progress_base = performance.now() / 1000 - data.position / (1000000 * this.playback_rate);
                this.launchUpdateTimer();
            }
            if (!data.running && this.update_progress !== null) {
                cancelAnimationFrame(this.update_progress);
                this.update_progress = null;
            }
        });
        eventSource.addEventListener("end", () => {
            this.parentNode.removeChild(this);
        });
        return eventSource;
    }

    launchUpdateTimer() {
        this.update_progress = requestAnimationFrame(() => {
            this.child_progress.value = (performance.now() / 1000 - this.progress_base) * 1000000 * this.playback_rate;
            this.launchUpdateTimer();
        });
    }
}

customElements.define("media-widget", MediaWidget);

fetch("/list")
    .then(res => res.json())
    .then(data => {
        const list = document.getElementById("list");
        for (let media of data) {
            const media_widget = document.createElement("media-widget");
            media_widget.setAttribute("media-id", media);
            list.appendChild(media_widget);
        }
    });
