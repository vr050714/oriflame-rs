# Русский. Те же сообщения, что и в en.ftl.

## Menu

app-tagline = Интриги за корону — против компьютера
players = Игроки
player-count = { $count }  (вы + { $pcs } ПК)
strength = Сила компьютера
level-easy = Лёгкий
level-medium = Средний
level-hard = Сложный
speed-normal = Быстро
speed-slow = Медленно
start-game = Начать игру

## Players

player-you = Вы
player-pc = ПК { $number }

## Cards

card-archer = Лучник
card-archer-text = Устраните первую или последнюю карту очереди.
card-soldier = Солдат
card-soldier-text = Устраните соседнюю карту.
card-spy = Шпион
card-spy-text = Украдите 1 очко влияния у игрока, чья карта соседствует с этой.
card-heir = Наследник
card-heir-text = Получите 2 очка влияния, если не раскрыт ни один другой Наследник.
card-assassination = Убийство
card-assassination-text = Устраните любую карту. Сбросьте Убийство.
card-royal_decree = Королевский указ
card-royal_decree-text = Переместите карту в любое место очереди. Сбросьте Королевский указ.
card-lord = Лорд
card-lord-text = Получите 1 очко влияния и ещё по 1 за каждую соседнюю карту вашей семьи.
card-shapeshifter = Оборотень
card-shapeshifter-text = Скопируйте способность соседнего раскрытого персонажа.
card-ambush = Засада
card-ambush-text = Если соперник устранил её закрытой: получите 4 и сбросьте атакующую карту. Если вы раскрыли её сами: получите 1. Сбросьте Засаду.
card-conspiracy = Заговор
card-conspiracy-text = Получите вдвое больше влияния, чем лежит на этой карте. Сбросьте Заговор.

## Board

round-of = Раунд { $round } из { $rounds }
menu = Меню
influence = { $amount } { $amount ->
    [one] очко
    [few] очка
   *[other] очков
} влияния
in-hand = { $count } в руке
queue-empty = Очередь пуста.
face-down = закрыта
hidden = (скрыта)
place = Положить
here = сюда
move-here = Сюда
reveal = Раскрыть
keep-hidden = Не раскрывать (+1)
pick-another = Выбрать другую карту
your-hand = Ваши карты
pick-to-see = Выберите карту в руке, чтобы узнать, что она делает.
copying = { $card } (копирует: { $copied })

## What to do now

game-is-over = Игра окончена.
thinking = { $player } думает…
pick-hand-card = Ваш ход: выберите карту из руки.
pick-place = Выберите, куда положить карту «{ $card }». Она ляжет закрытой.
reveal-or-not = Ваша карта «{ $card }» (отмечена): раскрыть её или оставить закрытой и добавить на неё 1 очко влияния?
decree-pick-card = Королевский указ: выберите карту, которую нужно переместить.
decree-pick-place = Королевский указ: выберите новое место для карты.
pick-target = { $card }: выберите выделенную карту.

## Story of the game. Card names stay in the nominative: "карту { $card }".

story-actor = { $lead ->
    [yes] { $player_you ->
        [yes] Ваша карта { $card }
       *[other] Карта { $card } игрока { $player }
    }
   *[other] Она
}
story-amount = { $amount } { $amount ->
    [one] очко
    [few] очка
   *[other] очков
} влияния
story-round = Начинается раунд { $round }.
story-placed-left = { $player_you ->
    [yes] Вы кладёте
   *[other] { $player } кладёт
} карту { $card } в начало очереди.
story-placed-right = { $player_you ->
    [yes] Вы кладёте
   *[other] { $player } кладёт
} карту { $card } в конец очереди.
story-placed-stack = { $player_you ->
    [yes] Вы кладёте
   *[other] { $player } кладёт
} карту { $card } поверх карты { $target }.
story-kept-hidden = { $player_you ->
    [yes] Вы не раскрываете
   *[other] { $player } не раскрывает
} карту { $card }, и ставка на ней растёт до { $amount }.
story-revealed = { $player_you ->
    [yes] Вы раскрываете
   *[other] { $player } раскрывает
} карту { $card }.
story-revealed-gain = { $player_you ->
    [yes] Вы раскрываете карту { $card } и забираете
   *[other] { $player } раскрывает карту { $card } и забирает
} { story-amount }.
story-gained = { story-actor } приносит { story-amount }.
story-stole = { story-actor } тянется мимо карты { $target } и крадёт 1 очко влияния у { $victim_you ->
    [yes] вас
   *[other] { $victim }
}.
story-copied = { story-actor } принимает облик { $owner_you ->
    [yes] вашей карты { $target }
   *[other] карты { $target } игрока { $owner }
}.
story-eliminated = { story-actor } устраняет { $owner_you ->
    [yes] вашу карту { $target }
   *[other] карту { $target } игрока { $owner }
}, и { $player_you ->
    [yes] вы получаете
   *[other] { $player } получает
} 1 очко влияния.
story-eliminated-self = { story-actor } устраняет сама себя, и { $player_you ->
    [yes] вы получаете
   *[other] { $player } получает
} 1 очко влияния.
story-ambushed = { story-actor } нападает на { $owner_you ->
    [yes] вашу карту { $target }
   *[other] карту { $target } игрока { $owner }
} и попадает в ловушку: { $owner_you ->
    [yes] вы получаете
   *[other] { $owner } получает
} { story-amount }, { $player_you ->
    [yes] вы
   *[other] { $player }
} — 1 очко.
story-ambushed-own = { story-actor } попадает в { $player_you ->
    [yes] вашу собственную
   *[other] собственную
} карту { $target }, и это приносит { story-amount }.
story-discarded = { story-actor } уходит в сброс.
story-moved = { story-actor } переносит карту { $target } на { $position }-е место в очереди.
story-tally-final = К концу раунда:
story-tally-now = Пока что:
story-tally-item = { $player }: { $total } ({ $change })

## Game log on the board and in the log window.

log-error = Ошибка: { $error }
log-hidden-card = рубашкой вверх
log-change = { $player } { $amount }
log-empty = В этом раунде ещё ничего не произошло.
log-open = Журнал
log-title = История партии
log-round-tab = Раунд { $round }
log-close = Закрыть
log-paused = Пока история открыта, игра ждёт.

## Labels on cards while an action is shown

mark-copied = скопирована
mark-eliminated = устранена
mark-ambush = Засада! +{ $gained }
mark-discarded = сброшена
mark-moves = перемещается

## Game over

you-win = Вы победили!
you-share-win = Вы делите победу!
winners = Победа: { $names }
and = { " " }и{ " " }
tie-break = При равенстве очков побеждает тот, у кого больше карт осталось в очереди.
back-to-menu = В меню
