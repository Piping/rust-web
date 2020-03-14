drop table if exists todo_item;
drop table if exists todo_list;

create table todo_list (
    id serial primary key,
    title varchar(150) not null
);

create table todo_item (
    id serial primary key,
    title varchar(150) not null,
    checked boolean not null default false,
    list_id integer not null,
    foreign key (list_id) references todo_list(id)
);

insert into todo_list (title) values ('List 1'), ('List 2');
insert into todo_item (title, list_id) values ('Buy Books', 1), ('Buy Food', 2), ('Buy Shoes', 1);
