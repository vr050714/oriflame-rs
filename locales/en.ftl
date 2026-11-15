# English. The reference language: every other file has the same messages.
# Syntax: https://projectfluent.org/fluent/guide/

## Menu

app-tagline = Intrigue for the crown, against the computer
players = Players
player-count = { $count }  (you + { $pcs } PC)
strength = Computer strength
level-easy = Easy
level-medium = Medium
level-hard = Hard
start-game = Start game
speed-normal = Fast
speed-slow = Slow

## Players

player-you = You
player-pc = PC { $number }

## Cards

card-archer = Archer
card-archer-text = Eliminate the first or last card of the queue.
card-soldier = Soldier
card-soldier-text = Eliminate an adjacent card.
card-spy = Spy
card-spy-text = Steal 1 influence from a player with an adjacent card.
card-heir = Heir
card-heir-text = Gain 2 influence if no other Heir is revealed.
card-assassination = Assassination
card-assassination-text = Eliminate any card. Discard Assassination.
card-royal_decree = Royal Decree
card-royal_decree-text = Move a card anywhere in the queue. Discard Royal Decree.
card-lord = Lord
card-lord-text = Gain 1 influence, plus 1 per adjacent card of your family.
card-shapeshifter = Shapeshifter
card-shapeshifter-text = Copy the ability of an adjacent revealed character.
card-ambush = Ambush
card-ambush-text = Eliminated face down by an opponent: gain 4 and discard the attacker. Revealed by you: gain 1. Discard Ambush.
card-conspiracy = Conspiracy
card-conspiracy-text = Gain double the influence on it. Discard Conspiracy.

## Board

round-of = Round { $round } of { $rounds }
menu = Menu
influence = { $amount } influence
in-hand = { $count } in hand
queue-empty = The queue is empty.
face-down = face down
hidden = (hidden)
place = Place
here = here
move-here = Here
reveal = Reveal
keep-hidden = Keep hidden (+1)
pick-another = Pick another card
your-hand = Your hand
pick-to-see = Pick a card from your hand to see what it does.
copying = { $card } (copying { $copied })

## What to do now

game-is-over = The game is over.
thinking = { $player } is thinking...
pick-hand-card = Your turn: pick a card from your hand.
pick-place = Pick where to place your { $card }. It goes face down.
reveal-or-not = Your { $card } (marked): reveal it, or keep it hidden and add 1 influence to it?
decree-pick-card = Royal Decree: pick the card to move.
decree-pick-place = Royal Decree: pick the new place for the card.
pick-target = { $card }: pick a highlighted card.

## Game log. Each line tells one event. { $card } (the card that acts) and
## { $target } (the card acted upon) are drawn as a miniature and the card's
## name, or as a hidden card, so keep them in a form that needs no grammatical case.

log-round = Round { $round } begins.
log-placed-left = { $player } placed { $card } at the start of the queue.
log-placed-right = { $player } placed { $card } at the end of the queue.
log-placed-stack = { $player } placed { $card } on top of { $target }.
log-kept-hidden = { $player } kept { $card } hidden; it now holds { $influence } influence.
log-revealed = { $player } revealed { $card }.
log-gained = { $player }: { $card } brought { $amount } influence.
log-stole = { $player }: { $card } stole 1 influence from { $victim } (next to { $target }).
log-copied = { $player }: { $card } copied { $target } ({ $owner }).
log-eliminated = { $player }: { $card } eliminated { $target } ({ $owner }).
log-eliminated-self = { $player }: { $card } eliminated itself.
log-ambushed = { $player }: { $card } attacked { $target } ({ $owner }) and fell into the ambush.
log-discarded = { $player } discarded { $card }.
log-moved = { $player }: { $card } moved { $target } ({ $owner }) to position { $position }.
log-error = Error: { $error }
log-hidden-card = a face-down card
log-change = { $player } { $amount }
log-total = Round total:
log-empty = Nothing has happened in this round yet.
log-open = Log
log-title = Game log
log-round-tab = Round { $round }
log-close = Close
log-paused = The game waits while the log is open.

## Labels on cards while an action is shown

mark-copied = copied
mark-eliminated = eliminated
mark-ambush = Ambush! +{ $gained }
mark-discarded = discarded
mark-moves = moves

## Game over

you-win = You win!
you-share-win = You share the win!
winners = { $names } won
and = { " " }and{ " " }
tie-break = Ties are broken by the number of cards left in the queue.
back-to-menu = Back to menu
