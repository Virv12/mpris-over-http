
const widget = document.getElementById('widget');
const title = document.getElementById('title');
const time = document.getElementById('time');

let update_timer = null;

async function set_metadata(req) {
    const res = await req;
    const data = await res.json();
    console.log(data);

    title.textContent = data.title;
    time.value = data.position / 1000000;
    time.max = data.length / 1000000;

    if (data.running) {
        if (update_timer === null) {
            update_timer = setInterval(() => {
                time.value += 1;
            }, 1000);
        }
    } else {
        if (update_timer !== null) {
            clearInterval(update_timer);
            update_timer = null;
        }
    }
}

widget.addEventListener('click', () => {
    set_metadata(fetch('/api/playpause'));
});

set_metadata(fetch('/api/metadata'));
